use chrono::{Datelike, Local, TimeZone, Utc};
use httparse::Status;
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_notification::NotificationExt;

const APP_SETTINGS_KEY: &str = "app_settings";
const TIMER_STATE_KEY: &str = "timer_state";
const TRAY_ID: &str = "pomodoro-tray";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TimerPhase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl std::fmt::Display for TimerPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            TimerPhase::Focus => "Focus",
            TimerPhase::ShortBreak => "Short break",
            TimerPhase::LongBreak => "Long break",
        };
        write!(f, "{label}")
    }
}

impl TimerPhase {
    fn as_db_value(&self) -> &'static str {
        match self {
            TimerPhase::Focus => "focus",
            TimerPhase::ShortBreak => "short_break",
            TimerPhase::LongBreak => "long_break",
        }
    }

    fn from_db_value(value: &str) -> AppResult<Self> {
        match value.trim_matches('"') {
            "focus" => Ok(TimerPhase::Focus),
            "short_break" => Ok(TimerPhase::ShortBreak),
            "long_break" => Ok(TimerPhase::LongBreak),
            other => Err(format!("unknown timer phase: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AppSettings {
    focus_min: i64,
    short_break_min: i64,
    long_break_min: i64,
    long_break_every: i64,
    theme: String,
    sound_enabled: bool,
    notifications_enabled: bool,
    remote_control_enabled: bool,
    remote_control_port: i64,
    remote_control_token: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            long_break_every: 4,
            theme: "light".to_string(),
            sound_enabled: true,
            notifications_enabled: true,
            remote_control_enabled: false,
            remote_control_port: 48484,
            remote_control_token: String::new(),
        }
    }
}

impl AppSettings {
    fn duration_for_phase_seconds(&self, phase: &TimerPhase) -> i64 {
        match phase {
            TimerPhase::Focus => self.focus_min * 60,
            TimerPhase::ShortBreak => self.short_break_min * 60,
            TimerPhase::LongBreak => self.long_break_min * 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsPatch {
    focus_min: Option<i64>,
    short_break_min: Option<i64>,
    long_break_min: Option<i64>,
    long_break_every: Option<i64>,
    theme: Option<String>,
    sound_enabled: Option<bool>,
    notifications_enabled: Option<bool>,
    remote_control_enabled: Option<bool>,
    remote_control_port: Option<i64>,
    remote_control_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimerState {
    phase: TimerPhase,
    remaining_seconds: i64,
    is_running: bool,
    cycle_index: i64,
    started_at: Option<i64>,
    phase_total_seconds: i64,
    interruptions: i64,
    current_project_id: Option<i64>,
    current_tag_ids: Vec<i64>,
    target_ends_at: Option<i64>,
}

impl TimerState {
    fn default_with_settings(settings: &AppSettings) -> Self {
        let phase = TimerPhase::Focus;
        let phase_total_seconds = settings.duration_for_phase_seconds(&phase);
        Self {
            phase,
            remaining_seconds: phase_total_seconds,
            is_running: false,
            cycle_index: 0,
            started_at: None,
            phase_total_seconds,
            interruptions: 0,
            current_project_id: None,
            current_tag_ids: Vec::new(),
            target_ends_at: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartTimerRequest {
    // Distinguish between:
    // - field missing: do not change current project (None)
    // - field present as null: clear current project (Some(None))
    // - field present as number: set current project (Some(Some(id)))
    project_id: Option<Option<i64>>,
    tag_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteSessionRequest {
    started_at: i64,
    ended_at: i64,
    phase: TimerPhase,
    duration_sec: i64,
    completed: bool,
    interruptions: i64,
    project_id: Option<i64>,
    tag_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionRecord {
    id: i64,
    started_at: i64,
    ended_at: i64,
    phase: TimerPhase,
    duration_sec: i64,
    completed: bool,
    interruptions: i64,
    project_id: Option<i64>,
    tag_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsRange {
    from: Option<i64>,
    to: Option<i64>,
    project_id: Option<i64>,
    tag_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsSummary {
    total_focus_sec: i64,
    completed_pomodoros: i64,
    streak_days: i64,
    interruptions: i64,
    avg_daily_focus_sec: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TimeseriesPoint {
    date: String,
    focus_seconds: i64,
    completed_pomodoros: i64,
    interruptions: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: i64,
    name: String,
    color: Option<String>,
    archived: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInput {
    id: Option<i64>,
    name: String,
    color: Option<String>,
    archived: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Tag {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagInput {
    id: Option<i64>,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    filename: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRange {
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetAllResult {
    settings: AppSettings,
    timer: TimerState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhaseCompletedEvent {
    completed_phase: TimerPhase,
    next_phase: TimerPhase,
}

struct AppModel {
    conn: Connection,
    settings: AppSettings,
    timer: TimerState,
}

struct RemoteServerHandle {
    port: u16,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

struct RemoteControlState {
    server: Option<RemoteServerHandle>,
}

struct AppState {
    model: Mutex<AppModel>,
    remote: Mutex<RemoteControlState>,
}

type AppResult<T> = Result<T, String>;

fn now_ts() -> i64 {
    Utc::now().timestamp()
}

fn lock_model<'a>(state: &'a State<'_, AppState>) -> AppResult<MutexGuard<'a, AppModel>> {
    state.model.lock().map_err(|e| e.to_string())
}

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

fn setup_tray(app: &AppHandle) -> AppResult<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Start / Pause", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let skip = MenuItem::with_id(app, "skip", "Skip phase", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let open = MenuItem::with_id(app, "open", "Open dashboard", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit =
        MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&toggle, &skip, &open, &quit]).map_err(|e| e.to_string())?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .title("Pomodoro")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let _ = tray_toggle_timer(app);
            }
            "skip" => {
                let _ = tray_skip_timer(app);
            }
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn tray_toggle_timer(app: &AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let timer = {
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        refresh_remaining(&mut model.timer);

        if model.timer.is_running {
            model.timer.is_running = false;
            model.timer.target_ends_at = None;
            if model.timer.phase == TimerPhase::Focus {
                model.timer.interruptions += 1;
            }
        } else {
            if model.timer.remaining_seconds <= 0 {
                model.timer.remaining_seconds = model.timer.phase_total_seconds;
            }
            if model.timer.started_at.is_none() {
                model.timer.started_at = Some(now_ts());
            }
            model.timer.is_running = true;
            model.timer.target_ends_at = Some(now_ts() + model.timer.remaining_seconds);
        }

        save_timer_state(&model.conn, &model.timer)?;
        model.timer.clone()
    };

    emit_timer_state(app, &timer);
    Ok(())
}

fn tray_skip_timer(app: &AppHandle) -> AppResult<()> {
    let (session, phase_event, timer) = {
        let state = app.state::<AppState>();
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        refresh_remaining(&mut model.timer);
        complete_and_advance(app, &mut model, false)?
    };

    let _ = app.emit("session://completed", &session);
    let _ = app.emit("timer://phase-completed", &phase_event);
    emit_timer_state(app, &timer);
    Ok(())
}

fn timer_start_inner(
    app: &AppHandle,
    state: &AppState,
    payload: Option<StartTimerRequest>,
) -> AppResult<TimerState> {
    let timer = {
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        refresh_remaining(&mut model.timer);

        if let Some(payload) = payload {
            if let Some(project_id) = payload.project_id {
                model.timer.current_project_id = project_id;
            }
            if let Some(tag_ids) = payload.tag_ids {
                model.timer.current_tag_ids = tag_ids;
            }
        }

        if model.timer.remaining_seconds <= 0 {
            model.timer.remaining_seconds = model.timer.phase_total_seconds;
        }
        if model.timer.started_at.is_none() {
            model.timer.started_at = Some(now_ts());
        }

        model.timer.is_running = true;
        model.timer.target_ends_at = Some(now_ts() + model.timer.remaining_seconds);

        save_timer_state(&model.conn, &model.timer)?;
        model.timer.clone()
    };

    emit_timer_state(app, &timer);
    Ok(timer)
}

fn timer_pause_inner(app: &AppHandle, state: &AppState) -> AppResult<TimerState> {
    let timer = {
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        refresh_remaining(&mut model.timer);
        if model.timer.phase == TimerPhase::Focus && model.timer.is_running {
            model.timer.interruptions += 1;
        }
        model.timer.is_running = false;
        model.timer.target_ends_at = None;
        save_timer_state(&model.conn, &model.timer)?;
        model.timer.clone()
    };

    emit_timer_state(app, &timer);
    Ok(timer)
}

fn timer_resume_inner(
    app: &AppHandle,
    state: &AppState,
    payload: Option<StartTimerRequest>,
) -> AppResult<TimerState> {
    let timer = {
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        if let Some(payload) = payload {
            if let Some(project_id) = payload.project_id {
                model.timer.current_project_id = project_id;
            }
            if let Some(tag_ids) = payload.tag_ids {
                model.timer.current_tag_ids = tag_ids;
            }
        }
        if model.timer.remaining_seconds <= 0 {
            model.timer.remaining_seconds = model.timer.phase_total_seconds;
        }
        if model.timer.started_at.is_none() {
            model.timer.started_at = Some(now_ts());
        }
        model.timer.is_running = true;
        model.timer.target_ends_at = Some(now_ts() + model.timer.remaining_seconds);
        save_timer_state(&model.conn, &model.timer)?;
        model.timer.clone()
    };

    emit_timer_state(app, &timer);
    Ok(timer)
}

fn timer_skip_inner(app: &AppHandle, state: &AppState) -> AppResult<TimerState> {
    let (session, phase_event, timer) = {
        let mut model = state.model.lock().map_err(|e| e.to_string())?;
        refresh_remaining(&mut model.timer);
        complete_and_advance(app, &mut model, false)?
    };

    let _ = app.emit("session://completed", &session);
    let _ = app.emit("timer://phase-completed", &phase_event);
    emit_timer_state(app, &timer);
    Ok(timer)
}

fn timer_get_state_inner(state: &AppState) -> AppResult<TimerState> {
    let mut model = state.model.lock().map_err(|e| e.to_string())?;
    refresh_remaining(&mut model.timer);
    Ok(model.timer.clone())
}

fn remote_stop(remote: &mut RemoteControlState) {
    if let Some(mut handle) = remote.server.take() {
        handle.stop.store(false, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }
}

fn remote_apply(app: &AppHandle, settings: &AppSettings) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut remote = state.remote.lock().map_err(|e| e.to_string())?;

    if !settings.remote_control_enabled {
        remote_stop(&mut remote);
        return Ok(());
    }

    let port = settings.remote_control_port as u16;
    let needs_restart = match remote.server.as_ref() {
        None => true,
        Some(handle) => handle.port != port,
    };

    if !needs_restart {
        return Ok(());
    }

    remote_stop(&mut remote);

    let stop = Arc::new(AtomicBool::new(true));
    let stop_thread = stop.clone();
    let app_handle = app.clone();

    let join = thread::spawn(move || remote_server_loop(app_handle, port, stop_thread));
    remote.server = Some(RemoteServerHandle {
        port,
        stop,
        join: Some(join),
    });

    Ok(())
}

fn header_value<'a>(headers: &'a [httparse::Header<'a>], name: &str) -> Option<&'a str> {
    for h in headers {
        if h.name.eq_ignore_ascii_case(name) {
            return std::str::from_utf8(h.value).ok();
        }
    }
    None
}

fn parse_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for part in query.split('&') {
        let mut it = part.splitn(2, '=');
        let k = it.next().unwrap_or("");
        if k == key {
            return Some(it.next().unwrap_or(""));
        }
    }
    None
}

fn split_path_query(path: &str) -> (&str, &str) {
    match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    }
}

fn write_response(stream: &mut std::net::TcpStream, code: &str, content_type: &str, body: &[u8]) {
    let headers = format!(
        "HTTP/1.1 {code}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, X-Pomodoro-Token\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body);
}

fn remote_html() -> String {
    // Minimal, mobile-friendly control page served from the Rust backend.
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Pomodoro Remote</title>
    <style>
      :root { color-scheme: light; }
      body { font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial; margin: 0; background: #0b1220; color: #e8eefc; }
      .wrap { max-width: 520px; margin: 0 auto; padding: 16px; }
      .card { background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.10); border-radius: 16px; padding: 16px; }
      .title { font-size: 18px; font-weight: 650; letter-spacing: 0.2px; margin: 0 0 10px; }
      .row { display: flex; gap: 10px; align-items: center; justify-content: space-between; }
      .big { font-size: 44px; font-weight: 750; letter-spacing: 0.6px; }
      .muted { color: rgba(232,238,252,0.72); font-size: 13px; }
      .btns { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 14px; }
      button { appearance: none; border: 1px solid rgba(255,255,255,0.14); background: rgba(255,255,255,0.08); color: #e8eefc; padding: 12px 14px; border-radius: 12px; font-size: 16px; font-weight: 650; }
      button:active { transform: translateY(1px); }
      button.primary { background: rgba(46, 160, 255, 0.22); border-color: rgba(46, 160, 255, 0.35); }
      button.danger { background: rgba(255, 77, 77, 0.18); border-color: rgba(255, 77, 77, 0.30); }
      .token { width: 100%; padding: 12px 14px; border-radius: 12px; border: 1px solid rgba(255,255,255,0.14); background: rgba(0,0,0,0.25); color: #e8eefc; font-size: 16px; }
      .sp { height: 12px; }
      a { color: #a9d1ff; }
    </style>
  </head>
  <body>
    <div class="wrap">
      <div class="card">
        <p class="title">Pomodoro Remote</p>
        <div id="auth">
          <p class="muted">Enter your token (from the macOS app Settings) to control the timer.</p>
          <input id="token" class="token" placeholder="Token" autocomplete="off" />
          <div class="sp"></div>
          <button class="primary" id="saveToken">Continue</button>
        </div>
        <div id="main" style="display:none">
          <div class="row">
            <div>
              <div class="muted" id="phase">...</div>
              <div class="big" id="time">--:--</div>
            </div>
            <div class="muted" id="status">...</div>
          </div>
          <div class="btns">
            <button class="primary" id="toggle">Start / Pause</button>
            <button class="danger" id="skip">Skip Phase</button>
          </div>
          <div class="sp"></div>
          <p class="muted">Tip: you can bookmark this page. Token is stored in the URL as <code>?token=...</code>.</p>
        </div>
      </div>
      <div class="sp"></div>
      <p class="muted">If this page does not load: ensure Remote Control is enabled in the desktop app Settings, and your iPhone and Mac are on the same Wi‑Fi.</p>
    </div>

    <script>
      const qs = new URLSearchParams(location.search);
      let token = qs.get("token") || "";

      const auth = document.getElementById("auth");
      const main = document.getElementById("main");
      const tokenInput = document.getElementById("token");
      const saveToken = document.getElementById("saveToken");

      function withTokenUrl(t) {
        const u = new URL(location.href);
        u.searchParams.set("token", t);
        return u.toString();
      }

      function showMain() { auth.style.display = "none"; main.style.display = "block"; }
      function showAuth() { auth.style.display = "block"; main.style.display = "none"; }

      if (token) showMain(); else showAuth();
      tokenInput.value = token;
      saveToken.addEventListener("click", () => {
        const t = (tokenInput.value || "").trim();
        if (!t) return;
        location.href = withTokenUrl(t);
      });

      async function api(path, method) {
        const res = await fetch(path, {
          method,
          headers: { "X-Pomodoro-Token": token }
        });
        if (res.status === 401) throw new Error("Unauthorized (bad token)");
        if (!res.ok) throw new Error("HTTP " + res.status);
        return res.json();
      }

      function phaseLabel(p) {
        if (p === "focus") return "Focus";
        if (p === "short_break") return "Short break";
        if (p === "long_break") return "Long break";
        return p;
      }

      function fmt(sec) {
        const m = Math.floor(sec / 60);
        const s = sec % 60;
        return String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0");
      }

      async function refresh() {
        if (!token) return;
        try {
          const st = await api("/api/state", "GET");
          document.getElementById("phase").textContent = phaseLabel(st.phase);
          document.getElementById("time").textContent = fmt(st.remainingSeconds);
          document.getElementById("status").textContent = st.isRunning ? "Running" : "Paused";
        } catch (e) {
          document.getElementById("status").textContent = String(e.message || e);
        }
      }

      document.getElementById("toggle").addEventListener("click", async () => {
        try { await api("/api/toggle", "POST"); } finally { await refresh(); }
      });
      document.getElementById("skip").addEventListener("click", async () => {
        try { await api("/api/skip", "POST"); } finally { await refresh(); }
      });

      refresh();
      setInterval(refresh, 1000);
    </script>
  </body>
</html>
"#
        .to_string()
}

fn remote_handle_connection(app: &AppHandle, mut stream: std::net::TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut buf = vec![0u8; 8192];
    let mut filled = 0usize;
    let mut header_end: Option<usize> = None;

    // Read until headers complete or size limit hit.
    while filled < buf.len() {
        match stream.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => {
                filled += n;
                if let Some(pos) = buf[..filled].windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(pos + 4);
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let header_end = match header_end {
        Some(v) => v,
        None => {
            write_response(
                &mut stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                b"bad request",
            );
            return;
        }
    };

    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let _parsed = match req.parse(&buf[..filled]) {
        Ok(Status::Complete(n)) => n,
        _ => {
            write_response(
                &mut stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                b"bad request",
            );
            return;
        }
    };

    let method = req.method.unwrap_or("");
    let path_raw = req.path.unwrap_or("/");
    let (path, query) = split_path_query(path_raw);

    let content_length = header_value(req.headers, "Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = Vec::new();
    if content_length > 0 {
        // We may already have some body bytes in the initial buffer.
        let already = filled.saturating_sub(header_end);
        if already > 0 {
            body.extend_from_slice(&buf[header_end..filled]);
        }

        while body.len() < content_length {
            let mut chunk = vec![0u8; (content_length - body.len()).min(4096)];
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
    }

    if method.eq_ignore_ascii_case("OPTIONS") {
        write_response(
            &mut stream,
            "204 No Content",
            "text/plain; charset=utf-8",
            b"",
        );
        return;
    }

    // Snapshot settings for auth/enable checks.
    let (remote_enabled, token_expected) = {
        let state = app.state::<AppState>();
        let model = match state.model.lock() {
            Ok(m) => m,
            Err(_) => {
                write_response(
                    &mut stream,
                    "500 Internal Server Error",
                    "text/plain; charset=utf-8",
                    b"error",
                );
                return;
            }
        };
        (
            model.settings.remote_control_enabled,
            model.settings.remote_control_token.clone(),
        )
    };

    if !remote_enabled {
        write_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
        );
        return;
    }

    // Serve the control page without requiring auth, so users can load it and paste a token.
    // All API endpoints remain token-protected.
    if method.eq_ignore_ascii_case("GET") && path == "/" {
        let html = remote_html();
        write_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            html.as_bytes(),
        );
        return;
    }

    let token_got = header_value(req.headers, "X-Pomodoro-Token")
        .or_else(|| parse_query_param(query, "token"))
        .unwrap_or("");
    if token_got != token_expected {
        write_response(
            &mut stream,
            "401 Unauthorized",
            "text/plain; charset=utf-8",
            b"unauthorized",
        );
        return;
    }

    // API routes.
    let state = app.state::<AppState>();
    let json = match (method, path) {
        ("GET", "/api/state") => match timer_get_state_inner(state.inner()) {
            Ok(v) => serde_json::to_vec(&v).ok(),
            Err(e) => {
                Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
            }
        },
        ("POST", "/api/toggle") => {
            let current = timer_get_state_inner(state.inner());
            let next = match current {
                Ok(st) => {
                    if st.is_running {
                        timer_pause_inner(app, state.inner())
                    } else if st.started_at.is_some() {
                        timer_resume_inner(app, state.inner(), None)
                    } else {
                        timer_start_inner(app, state.inner(), None)
                    }
                }
                Err(e) => Err(e),
            };
            match next {
                Ok(v) => serde_json::to_vec(&v).ok(),
                Err(e) => {
                    Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
                }
            }
        }
        ("POST", "/api/start") => {
            let payload = serde_json::from_slice::<StartTimerRequest>(&body).ok();
            match timer_start_inner(app, state.inner(), payload) {
                Ok(v) => serde_json::to_vec(&v).ok(),
                Err(e) => {
                    Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
                }
            }
        }
        ("POST", "/api/pause") => match timer_pause_inner(app, state.inner()) {
            Ok(v) => serde_json::to_vec(&v).ok(),
            Err(e) => {
                Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
            }
        },
        ("POST", "/api/resume") => {
            let payload = serde_json::from_slice::<StartTimerRequest>(&body).ok();
            match timer_resume_inner(app, state.inner(), payload) {
                Ok(v) => serde_json::to_vec(&v).ok(),
                Err(e) => {
                    Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
                }
            }
        }
        ("POST", "/api/skip") => match timer_skip_inner(app, state.inner()) {
            Ok(v) => serde_json::to_vec(&v).ok(),
            Err(e) => {
                Some(serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default())
            }
        },
        _ => None,
    };

    match json {
        Some(body) if !body.is_empty() => write_response(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            &body,
        ),
        Some(_) => write_response(
            &mut stream,
            "500 Internal Server Error",
            "text/plain; charset=utf-8",
            b"error",
        ),
        None => write_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
        ),
    }
}

fn remote_server_loop(app: AppHandle, port: u16, stop: Arc<AtomicBool>) {
    let addr = format!("0.0.0.0:{port}");
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("remote control server bind failed on {addr}: {e}");
            return;
        }
    };
    let _ = listener.set_nonblocking(true);

    while stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                remote_handle_connection(&app, stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

fn spawn_timer_worker(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));

        let mut should_emit = false;
        let mut emit_state: Option<TimerState> = None;
        let mut session_event: Option<SessionRecord> = None;
        let mut phase_event: Option<PhaseCompletedEvent> = None;

        {
            let state = app.state::<AppState>();
            let lock = state.model.lock();
            let mut model = match lock {
                Ok(guard) => guard,
                Err(_) => continue,
            };

            if !model.timer.is_running {
                continue;
            }

            let before = model.timer.remaining_seconds;
            refresh_remaining(&mut model.timer);

            if model.timer.remaining_seconds <= 0 {
                if let Ok((session, phase, timer)) = complete_and_advance(&app, &mut model, true) {
                    session_event = Some(session);
                    phase_event = Some(phase);
                    emit_state = Some(timer);
                    should_emit = true;
                }
            } else if model.timer.remaining_seconds != before {
                let _ = save_timer_state(&model.conn, &model.timer);
                emit_state = Some(model.timer.clone());
                should_emit = true;
            }
        }

        if should_emit {
            if let Some(session) = session_event {
                let _ = app.emit("session://completed", &session);
            }
            if let Some(phase) = phase_event {
                let _ = app.emit("timer://phase-completed", &phase);
            }
            if let Some(timer) = emit_state {
                emit_timer_state(&app, &timer);
            }
        }
    });
}

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
    let _ = remote_apply(&app, &settings);

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

    let _ = remote_apply(&app, &settings);
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
    Ok(socket.local_addr().map_err(|e| e.to_string())?.ip().to_string())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
            let db_path = app_dir.join("pomodoro.db");
            let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

            init_database(&conn)?;
            let settings = load_or_create_settings(&conn)?;
            let timer = load_or_create_timer(&conn, &settings)?;

            app.manage(AppState {
                model: Mutex::new(AppModel {
                    conn,
                    settings,
                    timer,
                }),
                remote: Mutex::new(RemoteControlState { server: None }),
            });

            setup_tray(app.handle())?;

            {
                let state = app.state::<AppState>();
                let model = state.model.lock().map_err(|e| e.to_string())?;
                update_tray_title(app.handle(), &model.timer);
            }

            spawn_timer_worker(app.handle().clone());

            // Remote control server (optional; disabled by default).
            {
                let state = app.state::<AppState>();
                let model = state.model.lock().map_err(|e| e.to_string())?;
                let _ = remote_apply(app.handle(), &model.settings);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            timer_start,
            timer_pause,
            timer_resume,
            timer_skip,
            timer_get_state,
            timer_set_context,
            session_complete,
            analytics_get_summary,
            analytics_get_timeseries,
            projects_list,
            projects_upsert,
            tags_list,
            tags_upsert,
            export_csv,
            export_json,
            settings_get,
            settings_update,
            reset_all_data,
            session_history,
            get_local_ip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> AppSettings {
        AppSettings {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            long_break_every: 4,
            theme: "light".to_string(),
            sound_enabled: true,
            notifications_enabled: true,
            remote_control_enabled: false,
            remote_control_port: 48484,
            remote_control_token: "testtoken".to_string(),
        }
    }

    #[test]
    fn advances_to_long_break_every_fourth_cycle() {
        let settings = sample_settings();
        let mut timer = TimerState::default_with_settings(&settings);

        timer.phase = TimerPhase::Focus;
        timer.cycle_index = 3;
        advance_timer(&mut timer, &settings);

        assert_eq!(timer.phase, TimerPhase::LongBreak);
        assert_eq!(timer.cycle_index, 4);
    }

    #[test]
    fn streak_counts_contiguous_days() {
        let now = now_ts();
        let day = 86_400;

        let sessions = vec![
            SessionRecord {
                id: 1,
                started_at: now - 100,
                ended_at: now - 50,
                phase: TimerPhase::Focus,
                duration_sec: 1500,
                completed: true,
                interruptions: 0,
                project_id: None,
                tag_ids: vec![],
            },
            SessionRecord {
                id: 2,
                started_at: now - day - 100,
                ended_at: now - day - 50,
                phase: TimerPhase::Focus,
                duration_sec: 1500,
                completed: true,
                interruptions: 0,
                project_id: None,
                tag_ids: vec![],
            },
        ];

        assert!(calculate_streak_days(&sessions) >= 2);
    }
}
