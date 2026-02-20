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

