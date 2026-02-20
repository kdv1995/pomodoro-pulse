#[tauri::command]
fn timer_start(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: Option<StartTimerRequest>,
) -> AppResult<TimerState> {
    timer_start_inner(&app, state.inner(), payload)
}

#[tauri::command]
fn timer_pause(app: AppHandle, state: State<'_, AppState>) -> AppResult<TimerState> {
    timer_pause_inner(&app, state.inner())
}

#[tauri::command]
fn timer_resume(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: Option<StartTimerRequest>,
) -> AppResult<TimerState> {
    timer_resume_inner(&app, state.inner(), payload)
}

#[tauri::command]
fn timer_skip(app: AppHandle, state: State<'_, AppState>) -> AppResult<TimerState> {
    timer_skip_inner(&app, state.inner())
}

#[tauri::command]
fn timer_get_state(state: State<'_, AppState>) -> AppResult<TimerState> {
    timer_get_state_inner(state.inner())
}

#[tauri::command]
fn timer_set_context(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: StartTimerRequest,
) -> AppResult<TimerState> {
    let timer = {
        let mut model = lock_model(&state)?;
        refresh_remaining(&mut model.timer);

        if let Some(project_id) = payload.project_id {
            model.timer.current_project_id = project_id;
        }
        if let Some(tag_ids) = payload.tag_ids {
            model.timer.current_tag_ids = tag_ids;
        }

        save_timer_state(&model.conn, &model.timer)?;
        model.timer.clone()
    };

    emit_timer_state(&app, &timer);
    Ok(timer)
}

#[tauri::command]
fn session_complete(
    payload: CompleteSessionRequest,
    state: State<'_, AppState>,
) -> AppResult<SessionRecord> {
    let model = lock_model(&state)?;

    model
        .conn
        .execute(
            "INSERT INTO sessions (started_at, ended_at, phase, duration_sec, completed, interruptions, project_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                payload.started_at,
                payload.ended_at,
                payload.phase.as_db_value(),
                payload.duration_sec,
                payload.completed as i64,
                payload.interruptions,
                payload.project_id,
            ],
        )
        .map_err(|e| e.to_string())?;

    let id = model.conn.last_insert_rowid();
    let tag_ids = payload.tag_ids.unwrap_or_default();
    for tag_id in &tag_ids {
        model
            .conn
            .execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(SessionRecord {
        id,
        started_at: payload.started_at,
        ended_at: payload.ended_at,
        phase: payload.phase,
        duration_sec: payload.duration_sec,
        completed: payload.completed,
        interruptions: payload.interruptions,
        project_id: payload.project_id,
        tag_ids,
    })
}

#[tauri::command]
fn analytics_get_summary(
    range: AnalyticsRange,
    state: State<'_, AppState>,
) -> AppResult<AnalyticsSummary> {
    let model = lock_model(&state)?;
    let sessions = fetch_sessions(&model.conn, &range)?;

    let mut total_focus_sec = 0;
    let mut completed_pomodoros = 0;
    let mut interruptions = 0;
    let mut days_with_focus = HashSet::new();

    for session in &sessions {
        if session.phase == TimerPhase::Focus {
            total_focus_sec += session.duration_sec;
            interruptions += session.interruptions;
            if session.completed {
                completed_pomodoros += 1;
            }
            if session.duration_sec > 0 {
                days_with_focus.insert(day_key(session.ended_at));
            }
        }
    }

    let avg_daily_focus_sec = if days_with_focus.is_empty() {
        0
    } else {
        total_focus_sec / days_with_focus.len() as i64
    };

    Ok(AnalyticsSummary {
        total_focus_sec,
        completed_pomodoros,
        streak_days: calculate_streak_days(&sessions),
        interruptions,
        avg_daily_focus_sec,
    })
}

#[tauri::command]
fn analytics_get_timeseries(
    range: AnalyticsRange,
    state: State<'_, AppState>,
) -> AppResult<Vec<TimeseriesPoint>> {
    let model = lock_model(&state)?;
    let sessions = fetch_sessions(&model.conn, &range)?;

    let mut by_day: BTreeMap<String, TimeseriesPoint> = BTreeMap::new();

    for session in sessions {
        if session.phase != TimerPhase::Focus {
            continue;
        }

        let key = day_key(session.ended_at);
        let entry = by_day.entry(key.clone()).or_insert(TimeseriesPoint {
            date: key,
            focus_seconds: 0,
            completed_pomodoros: 0,
            interruptions: 0,
        });

        entry.focus_seconds += session.duration_sec;
        entry.interruptions += session.interruptions;
        if session.completed {
            entry.completed_pomodoros += 1;
        }
    }

    Ok(by_day.into_values().collect())
}

#[tauri::command]
fn projects_list(state: State<'_, AppState>) -> AppResult<Vec<Project>> {
    let model = lock_model(&state)?;
    fetch_projects(&model.conn)
}

#[tauri::command]
fn projects_upsert(input: ProjectInput, state: State<'_, AppState>) -> AppResult<Project> {
    let model = lock_model(&state)?;

    let archived = input.archived.unwrap_or(false);
    let id = if let Some(id) = input.id {
        model
            .conn
            .execute(
                "UPDATE projects SET name = ?1, color = ?2, archived = ?3 WHERE id = ?4",
                params![input.name, input.color, archived as i64, id],
            )
            .map_err(|e| e.to_string())?;
        id
    } else {
        model
            .conn
            .execute(
                "INSERT INTO projects (name, color, archived, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![input.name, input.color, archived as i64, now_ts()],
            )
            .map_err(|e| e.to_string())?;
        model.conn.last_insert_rowid()
    };

    let project = model
        .conn
        .query_row(
            "SELECT id, name, color, archived FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    archived: row.get::<_, i64>(3)? == 1,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
fn tags_list(state: State<'_, AppState>) -> AppResult<Vec<Tag>> {
    let model = lock_model(&state)?;
    fetch_tags(&model.conn)
}

#[tauri::command]
fn tags_upsert(input: TagInput, state: State<'_, AppState>) -> AppResult<Tag> {
    let model = lock_model(&state)?;

    let id = if let Some(id) = input.id {
        model
            .conn
            .execute(
                "UPDATE tags SET name = ?1 WHERE id = ?2",
                params![input.name, id],
            )
            .map_err(|e| e.to_string())?;
        id
    } else {
        model
            .conn
            .execute(
                "INSERT INTO tags (name, created_at) VALUES (?1, ?2)",
                params![input.name, now_ts()],
            )
            .map_err(|e| e.to_string())?;
        model.conn.last_insert_rowid()
    };

    let tag = model
        .conn
        .query_row(
            "SELECT id, name FROM tags WHERE id = ?1",
            params![id],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(tag)
}

#[tauri::command]
fn export_csv(range: ExportRange, state: State<'_, AppState>) -> AppResult<ExportResult> {
    let model = lock_model(&state)?;
    let sessions = fetch_sessions(
        &model.conn,
        &AnalyticsRange {
            from: range.from,
            to: range.to,
            project_id: None,
            tag_id: None,
        },
    )?;

    let mut csv = String::from(
        "id,startedAt,endedAt,phase,durationSec,completed,interruptions,projectId,tagIds\n",
    );

    for s in sessions {
        let tag_ids = s
            .tag_ids
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(";");
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            s.id,
            s.started_at,
            s.ended_at,
            s.phase.as_db_value(),
            s.duration_sec,
            s.completed,
            s.interruptions,
            s.project_id
                .map(|v| v.to_string())
                .unwrap_or_else(|| "".to_string()),
            tag_ids
        ));
    }

    Ok(ExportResult {
        filename: format!("pomodoro-sessions-{}.csv", now_ts()),
        content: csv,
    })
}

#[tauri::command]
fn export_json(range: ExportRange, state: State<'_, AppState>) -> AppResult<ExportResult> {
    let model = lock_model(&state)?;

    let sessions = fetch_sessions(
        &model.conn,
        &AnalyticsRange {
            from: range.from,
            to: range.to,
            project_id: None,
            tag_id: None,
        },
    )?;
    let projects = fetch_projects(&model.conn)?;
    let tags = fetch_tags(&model.conn)?;

    let payload = serde_json::json!({
      "exportedAt": now_ts(),
      "settings": model.settings,
      "projects": projects,
      "tags": tags,
      "sessions": sessions
    });

    Ok(ExportResult {
        filename: format!("pomodoro-backup-{}.json", now_ts()),
        content: serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
fn settings_get(state: State<'_, AppState>) -> AppResult<AppSettings> {
    let model = lock_model(&state)?;
    Ok(model.settings.clone())
}

#[tauri::command]
fn settings_update(
    app: AppHandle,
    patch: AppSettingsPatch,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    let (settings, timer) = {
        let mut model = lock_model(&state)?;

        if let Some(v) = patch.focus_min {
            model.settings.focus_min = v;
        }
        if let Some(v) = patch.short_break_min {
            model.settings.short_break_min = v;
        }
        if let Some(v) = patch.long_break_min {
            model.settings.long_break_min = v;
        }
        if let Some(v) = patch.long_break_every {
            model.settings.long_break_every = v;
        }
        if let Some(v) = patch.theme {
            model.settings.theme = v.trim().to_lowercase();
        }
        if let Some(v) = patch.sound_enabled {
            model.settings.sound_enabled = v;
        }
        if let Some(v) = patch.notifications_enabled {
            model.settings.notifications_enabled = v;
        }
        if let Some(v) = patch.remote_control_enabled {
            model.settings.remote_control_enabled = v;
        }
        if let Some(v) = patch.remote_control_port {
            model.settings.remote_control_port = v;
        }
        if let Some(v) = patch.remote_control_token {
            model.settings.remote_control_token = v;
        }

        model.settings = normalize_settings(model.settings.clone());
        if model.settings.remote_control_token.trim().is_empty() {
            ensure_remote_token(&mut model.settings);
        }
        save_json_setting(&model.conn, APP_SETTINGS_KEY, &model.settings)?;

        // Keep the current phase duration in sync if timer is idle.
        if !model.timer.is_running {
            model.timer.phase_total_seconds = model
                .settings
                .duration_for_phase_seconds(&model.timer.phase);
            model.timer.remaining_seconds = model.timer.phase_total_seconds;
            model.timer.started_at = None;
            model.timer.target_ends_at = None;
            save_timer_state(&model.conn, &model.timer)?;
        }

        (model.settings.clone(), model.timer.clone())
    };

    // Start/stop/restart remote control server based on settings.
    remote_apply(&app, &settings)?;

    emit_timer_state(&app, &timer);
    Ok(settings)
}

#[tauri::command]
fn reset_all_data(app: AppHandle, state: State<'_, AppState>) -> AppResult<ResetAllResult> {
    let (settings, timer) = {
        let mut model = lock_model(&state)?;

        {
            let tx = model.conn.transaction().map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM session_tags", [])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM sessions", [])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM projects", [])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM tags", [])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM settings", [])
                .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM sqlite_sequence WHERE name IN ('projects', 'tags', 'sessions')",
                [],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }

        model.settings = normalize_settings(AppSettings::default());
        ensure_remote_token(&mut model.settings);
        model.timer = TimerState::default_with_settings(&model.settings);
        save_json_setting(&model.conn, APP_SETTINGS_KEY, &model.settings)?;
        save_timer_state(&model.conn, &model.timer)?;

        (model.settings.clone(), model.timer.clone())
    };

    remote_apply(&app, &settings)?;
    emit_timer_state(&app, &timer);
    Ok(ResetAllResult { settings, timer })
}

#[tauri::command]
fn session_history(
    range: AnalyticsRange,
    state: State<'_, AppState>,
) -> AppResult<Vec<SessionRecord>> {
    let model = lock_model(&state)?;
    fetch_sessions(&model.conn, &range)
}

#[tauri::command]
fn get_local_ip() -> Result<String, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
    Ok(socket
        .local_addr()
        .map_err(|e| e.to_string())?
        .ip()
        .to_string())
}

