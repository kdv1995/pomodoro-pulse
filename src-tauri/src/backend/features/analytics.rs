fn build_sessions_query(range: &AnalyticsRange) -> (String, Vec<Value>) {
    let mut query = String::from(
        "SELECT id, started_at, ended_at, phase, duration_sec, completed, interruptions, project_id FROM sessions WHERE 1 = 1",
    );
    let mut values: Vec<Value> = Vec::new();

    if let Some(from) = range.from {
        query.push_str(" AND ended_at >= ?");
        values.push(Value::Integer(from));
    }
    if let Some(to) = range.to {
        query.push_str(" AND ended_at <= ?");
        values.push(Value::Integer(to));
    }
    if let Some(project_id) = range.project_id {
        query.push_str(" AND project_id = ?");
        values.push(Value::Integer(project_id));
    }
    if let Some(tag_id) = range.tag_id {
        query.push_str(" AND EXISTS (SELECT 1 FROM session_tags st WHERE st.session_id = sessions.id AND st.tag_id = ?)");
        values.push(Value::Integer(tag_id));
    }

    query.push_str(" ORDER BY ended_at DESC");

    (query, values)
}

fn read_session_tags(conn: &Connection, session_id: i64) -> AppResult<Vec<i64>> {
    let mut stmt = conn
        .prepare("SELECT tag_id FROM session_tags WHERE session_id = ?1 ORDER BY tag_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![session_id], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|e| e.to_string())?);
    }
    Ok(tags)
}

fn phase_from_db(value: String) -> AppResult<TimerPhase> {
    TimerPhase::from_db_value(&value)
}

fn fetch_projects(conn: &Connection) -> AppResult<Vec<Project>> {
    let mut stmt = conn
        .prepare("SELECT id, name, color, archived FROM projects ORDER BY archived ASC, name ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                archived: row.get::<_, i64>(3)? == 1,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut projects = Vec::new();
    for row in rows {
        projects.push(row.map_err(|e| e.to_string())?);
    }

    Ok(projects)
}

fn fetch_tags(conn: &Connection) -> AppResult<Vec<Tag>> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM tags ORDER BY name ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|e| e.to_string())?);
    }

    Ok(tags)
}

fn fetch_sessions(conn: &Connection, range: &AnalyticsRange) -> AppResult<Vec<SessionRecord>> {
    let (query, values) = build_sessions_query(range);
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut sessions = Vec::new();
    for row in rows {
        let (
            id,
            started_at,
            ended_at,
            phase_raw,
            duration_sec,
            completed,
            interruptions,
            project_id,
        ) = row.map_err(|e| e.to_string())?;
        sessions.push(SessionRecord {
            id,
            started_at,
            ended_at,
            phase: phase_from_db(phase_raw)?,
            duration_sec,
            completed: completed == 1,
            interruptions,
            project_id,
            tag_ids: read_session_tags(conn, id)?,
        });
    }

    Ok(sessions)
}

fn day_key(timestamp: i64) -> String {
    let dt = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(|| Local::now());
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

fn calculate_streak_days(sessions: &[SessionRecord]) -> i64 {
    let mut focus_days = HashSet::new();
    for session in sessions {
        if session.phase == TimerPhase::Focus && session.duration_sec > 0 {
            focus_days.insert(day_key(session.ended_at));
        }
    }

    let mut streak = 0;
    let mut current = Local::now().date_naive();
    loop {
        let key = current.format("%Y-%m-%d").to_string();
        if focus_days.contains(&key) {
            streak += 1;
            match current.pred_opt() {
                Some(prev) => current = prev,
                None => break,
            }
        } else {
            break;
        }
    }

    streak
}

