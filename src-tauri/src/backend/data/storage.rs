fn init_database(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at INTEGER NOT NULL,
            ended_at INTEGER NOT NULL,
            phase TEXT NOT NULL,
            duration_sec INTEGER NOT NULL,
            completed INTEGER NOT NULL,
            interruptions INTEGER NOT NULL DEFAULT 0,
            project_id INTEGER,
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS session_tags (
            session_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (session_id, tag_id),
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_ended_at ON sessions(ended_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
        CREATE INDEX IF NOT EXISTS idx_session_tags_tag_id ON session_tags(tag_id);
        "#,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn save_json_setting<T: Serialize>(conn: &Connection, key: &str, value: &T) -> AppResult<()> {
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_json_setting<T: for<'de> Deserialize<'de>>(
    conn: &Connection,
    key: &str,
) -> AppResult<Option<T>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match value {
        Some(raw) => {
            let parsed = serde_json::from_str::<T>(&raw).map_err(|e| e.to_string())?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    settings.focus_min = settings.focus_min.clamp(1, 180);
    settings.short_break_min = settings.short_break_min.clamp(1, 60);
    settings.long_break_min = settings.long_break_min.clamp(1, 90);
    settings.long_break_every = settings.long_break_every.clamp(2, 10);
    settings.theme = match settings.theme.as_str() {
        "dark" => "dark".to_string(),
        _ => "light".to_string(),
    };
    settings.remote_control_port = settings.remote_control_port.clamp(1024, 65535);
    settings
}

fn generate_remote_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn ensure_remote_token(settings: &mut AppSettings) {
    if settings.remote_control_token.trim().is_empty() {
        settings.remote_control_token = generate_remote_token();
    }
}

fn load_or_create_settings(conn: &Connection) -> AppResult<AppSettings> {
    let settings = load_json_setting::<AppSettings>(conn, APP_SETTINGS_KEY)?
        .unwrap_or_else(AppSettings::default);
    let mut settings = normalize_settings(settings);
    ensure_remote_token(&mut settings);
    save_json_setting(conn, APP_SETTINGS_KEY, &settings)?;
    Ok(settings)
}

fn normalize_timer_state(mut timer: TimerState, settings: &AppSettings) -> TimerState {
    timer.phase_total_seconds = settings.duration_for_phase_seconds(&timer.phase);
    if timer.remaining_seconds <= 0 || timer.remaining_seconds > timer.phase_total_seconds {
        timer.remaining_seconds = timer.phase_total_seconds;
        timer.is_running = false;
        timer.target_ends_at = None;
    }
    if timer.cycle_index < 0 {
        timer.cycle_index = 0;
    }
    timer.interruptions = timer.interruptions.max(0);
    timer
}

fn load_or_create_timer(conn: &Connection, settings: &AppSettings) -> AppResult<TimerState> {
    let timer = load_json_setting::<TimerState>(conn, TIMER_STATE_KEY)?
        .unwrap_or_else(|| TimerState::default_with_settings(settings));
    let timer = normalize_timer_state(timer, settings);
    save_json_setting(conn, TIMER_STATE_KEY, &timer)?;
    Ok(timer)
}

fn save_timer_state(conn: &Connection, timer: &TimerState) -> AppResult<()> {
    save_json_setting(conn, TIMER_STATE_KEY, timer)
}

fn refresh_remaining(timer: &mut TimerState) {
    if timer.is_running {
        if let Some(target_ends_at) = timer.target_ends_at {
            timer.remaining_seconds = (target_ends_at - now_ts()).max(0);
        }
    }
}

fn format_seconds(seconds: i64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{minutes:02}:{secs:02}")
}

fn update_tray_title(app: &AppHandle, timer: &TimerState) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let status = if timer.is_running {
            "Running"
        } else {
            "Paused"
        };
        let title = format!(
            "{} {} {status}",
            timer.phase,
            format_seconds(timer.remaining_seconds)
        );
        let _ = tray.set_title(Some(&title));
    }
}

fn emit_timer_state(app: &AppHandle, timer: &TimerState) {
    let _ = app.emit("timer://state", timer);
    update_tray_title(app, timer);
}

fn record_session(
    conn: &Connection,
    timer: &TimerState,
    completed: bool,
    ended_at: i64,
) -> AppResult<SessionRecord> {
    let elapsed = if completed {
        timer.phase_total_seconds
    } else {
        (timer.phase_total_seconds - timer.remaining_seconds).clamp(0, timer.phase_total_seconds)
    };

    let started_at = timer
        .started_at
        .unwrap_or_else(|| ended_at - elapsed.max(1));

    let project_id = match timer.phase {
        TimerPhase::Focus => timer.current_project_id,
        _ => None,
    };

    conn.execute(
        "INSERT INTO sessions (started_at, ended_at, phase, duration_sec, completed, interruptions, project_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            started_at,
            ended_at,
            timer.phase.as_db_value(),
            elapsed,
            completed as i64,
            timer.interruptions,
            project_id,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let mut tag_ids = Vec::new();

    if timer.phase == TimerPhase::Focus {
        for tag_id in &timer.current_tag_ids {
            conn.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )
            .map_err(|e| e.to_string())?;
            tag_ids.push(*tag_id);
        }
    }

    Ok(SessionRecord {
        id,
        started_at,
        ended_at,
        phase: timer.phase.clone(),
        duration_sec: elapsed,
        completed,
        interruptions: timer.interruptions,
        project_id,
        tag_ids,
    })
}

fn advance_timer(timer: &mut TimerState, settings: &AppSettings) {
    let next_phase = match timer.phase {
        TimerPhase::Focus => {
            timer.cycle_index += 1;
            if timer.cycle_index % settings.long_break_every == 0 {
                TimerPhase::LongBreak
            } else {
                TimerPhase::ShortBreak
            }
        }
        TimerPhase::ShortBreak | TimerPhase::LongBreak => TimerPhase::Focus,
    };

    timer.phase = next_phase;
    timer.phase_total_seconds = settings.duration_for_phase_seconds(&timer.phase);
    timer.remaining_seconds = timer.phase_total_seconds;
    timer.is_running = false;
    timer.started_at = None;
    timer.target_ends_at = None;
    timer.interruptions = 0;
}

fn complete_and_advance(
    app: &AppHandle,
    model: &mut AppModel,
    completed: bool,
) -> AppResult<(SessionRecord, PhaseCompletedEvent, TimerState)> {
    let finished_phase = model.timer.phase.clone();
    let session = record_session(&model.conn, &model.timer, completed, now_ts())?;

    advance_timer(&mut model.timer, &model.settings);
    save_timer_state(&model.conn, &model.timer)?;

    let event = PhaseCompletedEvent {
        completed_phase: finished_phase,
        next_phase: model.timer.phase.clone(),
    };

    if model.settings.notifications_enabled {
        let body = format!(
            "{} complete. Next: {}",
            event.completed_phase, event.next_phase
        );
        let _ = app
            .notification()
            .builder()
            .title("Pomodoro update")
            .body(&body)
            .show();
    }

    Ok((session, event, model.timer.clone()))
}

