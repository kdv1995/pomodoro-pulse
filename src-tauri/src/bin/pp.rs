use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal, ExecutableCommand,
};
use serde_json::{json, Value};
use std::{
    env, fs,
    io::{self, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 48484;
const CONFIG_FILE_NAME: &str = ".pomodoro-pulse-pp.json";
const WATCH_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(100);

const HELP_TEXT: &str = r#"pp - Pomodoro Pulse HTTP CLI

Usage:
  pp <subcommand>
  pp help

Commands:
  status         Current phase, time to end, next phase
  watch          Realtime terminal status (alias: live)
  start          Start current phase (or resume paused phase)
  resume         Resume paused phase only
  skip           Skip current phase
  stop           Stop timer
  token <token>  Save token for next commands
  port <port>    Save port for next commands
  help           Show this help

Config:
  token and port are stored in ~/.pomodoro-pulse-pp.json
  host is fixed to 127.0.0.1
"#;

const AVAILABLE_COMMANDS: &str = r#"Available Commands:
  status         Current phase, time to end, next phase
  watch          Realtime terminal status (alias: live)
  start          Start current phase (or resume paused phase)
  resume         Resume paused phase only
  skip           Skip current phase
  stop           Stop timer
  token <token>  Save token for next commands
  port <port>    Save port for next commands
  help           Show this help
"#;

#[derive(Debug)]
struct CliError {
    message: String,
    exit_code: i32,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }
}

type CliResult<T> = Result<T, CliError>;

#[derive(Debug, Clone)]
struct Config {
    host: String,
    port: u16,
    token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            token: String::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Status,
    Watch,
    Start,
    Resume,
    Skip,
    Stop,
    Token(String),
    Port(u16),
}

fn parse_port(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().filter(|port| *port > 0)
}

fn parse_args(args: &[String]) -> CliResult<Command> {
    if args.is_empty() {
        return Ok(Command::Help);
    }

    let command = args[0].as_str();
    match command {
        "help" | "--help" | "-h" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Help)
        }
        "status" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Status)
        }
        "watch" | "live" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Watch)
        }
        "start" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Start)
        }
        "resume" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Resume)
        }
        "skip" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Skip)
        }
        "stop" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Stop)
        }
        "token" => {
            if args.len() != 2 {
                return Err(CliError::usage("Usage: pp token <token>"));
            }
            let token = args[1].trim();
            if token.is_empty() {
                return Err(CliError::usage("Token cannot be empty."));
            }
            Ok(Command::Token(token.to_string()))
        }
        "port" => {
            if args.len() != 2 {
                return Err(CliError::usage("Usage: pp port <port>"));
            }
            let value = parse_port(&args[1])
                .ok_or_else(|| CliError::usage(format!("Invalid port: {}", args[1])))?;
            Ok(Command::Port(value))
        }
        other => Err(CliError::usage(format!("Unknown command: {other}"))),
    }
}

fn config_path() -> CliResult<PathBuf> {
    if let Ok(path) = env::var("USERPROFILE") {
        if !path.trim().is_empty() {
            return Ok(Path::new(&path).join(CONFIG_FILE_NAME));
        }
    }
    if let Ok(path) = env::var("HOME") {
        if !path.trim().is_empty() {
            return Ok(Path::new(&path).join(CONFIG_FILE_NAME));
        }
    }
    Err(CliError::usage(
        "Unable to resolve home directory for config file.",
    ))
}

fn load_config(path: &Path) -> CliResult<Config> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => {
            return Err(CliError::usage(format!(
                "Failed to read config {}: {err}",
                path.display()
            )));
        }
    };

    let value: Value = serde_json::from_str(&content).map_err(|_| {
        CliError::usage(format!("Invalid config JSON in {}", path.to_string_lossy()))
    })?;

    let token = value
        .get("token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    let port = match value.get("port") {
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|p| u16::try_from(p).ok())
            .filter(|p| *p > 0)
            .unwrap_or(DEFAULT_PORT),
        Some(Value::String(s)) => parse_port(s).unwrap_or(DEFAULT_PORT),
        _ => DEFAULT_PORT,
    };

    Ok(Config {
        host: DEFAULT_HOST.to_string(),
        port,
        token,
    })
}

fn save_config(path: &Path, config: &Config) -> CliResult<()> {
    let body = json!({
      "token": config.token,
      "port": config.port
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|e| CliError::usage(format!("Failed to encode config: {e}")))?;
    fs::write(path, format!("{encoded}\n"))
        .map_err(|e| CliError::usage(format!("Failed to write config {}: {e}", path.display())))
}

fn phase_label(phase: &str) -> String {
    match phase {
        "focus" => "Focus".to_string(),
        "short_break" => "Short break".to_string(),
        "long_break" => "Long break".to_string(),
        _ => phase.to_string(),
    }
}

fn format_time(seconds: i64) -> String {
    let safe = seconds.max(0);
    let hours = safe / 3600;
    let minutes = (safe % 3600) / 60;
    let secs = safe % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

fn render_large_glyph(ch: char) -> [&'static str; 5] {
    match ch {
        '0' => [" ### ", "#   #", "#   #", "#   #", " ### "],
        '1' => ["  #  ", " ##  ", "  #  ", "  #  ", " ### "],
        '2' => [" ### ", "#   #", "   # ", "  #  ", "#####"],
        '3' => [" ### ", "    #", " ### ", "    #", " ### "],
        '4' => ["#   #", "#   #", "#####", "    #", "    #"],
        '5' => ["#####", "#    ", "#### ", "    #", "#### "],
        '6' => [" ### ", "#    ", "#### ", "#   #", " ### "],
        '7' => ["#####", "    #", "   # ", "  #  ", " #   "],
        '8' => [" ### ", "#   #", " ### ", "#   #", " ### "],
        '9' => [" ### ", "#   #", " ####", "    #", " ### "],
        ':' => ["   ", " # ", "   ", " # ", "   "],
        ' ' => [" ", " ", " ", " ", " "],
        _ => ["????", "????", "????", "????", "????"],
    }
}

fn render_large_text(value: &str) -> String {
    let mut lines = vec![
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for ch in value.chars() {
        let glyph = render_large_glyph(ch);
        for (index, part) in glyph.iter().enumerate() {
            if !lines[index].is_empty() {
                lines[index].push(' ');
            }
            lines[index].push_str(part);
        }
    }
    lines.join("\n")
}

fn state_is_running(state: &Value) -> bool {
    state
        .get("isRunning")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn state_has_started(state: &Value) -> bool {
    !state.get("startedAt").map(Value::is_null).unwrap_or(true)
}

#[derive(Debug, PartialEq, Eq)]
enum StartAction {
    AlreadyRunning,
    ResumePaused,
    StartFresh,
}

fn decide_start_action(state: &Value) -> StartAction {
    if state_is_running(state) {
        StartAction::AlreadyRunning
    } else if state_has_started(state) {
        StartAction::ResumePaused
    } else {
        StartAction::StartFresh
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ResumeAction {
    AlreadyRunning,
    ResumePaused,
    NoPausedSession,
}

fn decide_resume_action(state: &Value) -> ResumeAction {
    if state_is_running(state) {
        ResumeAction::AlreadyRunning
    } else if state_has_started(state) {
        ResumeAction::ResumePaused
    } else {
        ResumeAction::NoPausedSession
    }
}

fn parse_http_response(raw: &[u8]) -> CliResult<(u16, Vec<u8>)> {
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| CliError::runtime("Invalid HTTP response from server."))?;

    let headers = &raw[..header_end];
    let body = raw[header_end + 4..].to_vec();

    let status_line_end = headers
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(headers.len());
    let status_line = std::str::from_utf8(&headers[..status_line_end])
        .map_err(|_| CliError::runtime("Invalid HTTP status line."))?;

    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| CliError::runtime("HTTP status code missing."))?
        .parse::<u16>()
        .map_err(|_| CliError::runtime("Invalid HTTP status code."))?;

    Ok((status_code, body))
}

fn request_json(config: &Config, method: &str, path: &str) -> CliResult<Value> {
    let addr = format!("{}:{}", config.host, config.port);
    let mut stream = TcpStream::connect(&addr).map_err(|_| {
        CliError::runtime(format!(
            "Connection failed: http://{}:{}{}. Ensure the app is running and Remote Control is enabled.",
            config.host, config.port, path
        ))
    })?;

    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nX-Pomodoro-Token: {token}\r\nContent-Length: 0\r\n\r\n",
        host = config.host,
        port = config.port,
        token = config.token
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|_| CliError::runtime("Failed to send request."))?;

    let mut buffer = Vec::new();
    stream
        .read_to_end(&mut buffer)
        .map_err(|_| CliError::runtime("Failed to read server response."))?;

    let (status_code, body_bytes) = parse_http_response(&buffer)?;
    let body = if body_bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice::<Value>(&body_bytes)
            .map_err(|_| CliError::runtime("Invalid JSON response from server."))?
    };

    if status_code >= 400 {
        let reason = body
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status_code}"));
        return Err(CliError::runtime(format!("Request failed: {reason}")));
    }

    if let Some(error) = body.get("error").and_then(Value::as_str) {
        return Err(CliError::runtime(format!("Server error: {error}")));
    }

    Ok(body)
}

fn run_status(config: &Config) -> CliResult<()> {
    let state = request_json(config, "GET", "/api/state")?;
    let phase = state
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let remaining = state
        .get("remainingSeconds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let next_phase = state
        .get("nextPhase")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    println!("Current phase: {}", phase_label(phase));
    println!("Time to end: {}", format_time(remaining));
    println!("Next phase: {}", phase_label(next_phase));
    Ok(())
}

fn run_start(config: &Config) -> CliResult<()> {
    let state = request_json(config, "GET", "/api/state")?;
    match decide_start_action(&state) {
        StartAction::AlreadyRunning => {
            println!("Timer is already running.");
        }
        StartAction::ResumePaused => {
            request_json(config, "POST", "/api/resume")?;
            println!("Timer resumed.");
        }
        StartAction::StartFresh => {
            request_json(config, "POST", "/api/start")?;
            println!("Timer started.");
        }
    }
    Ok(())
}

fn run_resume(config: &Config) -> CliResult<()> {
    let state = request_json(config, "GET", "/api/state")?;
    match decide_resume_action(&state) {
        ResumeAction::AlreadyRunning => {
            println!("Timer is already running.");
        }
        ResumeAction::ResumePaused => {
            request_json(config, "POST", "/api/resume")?;
            println!("Timer resumed.");
        }
        ResumeAction::NoPausedSession => {
            println!("No paused timer to resume. Use `pp start` to start a new session.");
        }
    }
    Ok(())
}

struct TerminalModeGuard;

impl TerminalModeGuard {
    fn start() -> CliResult<Self> {
        terminal::enable_raw_mode()
            .map_err(|e| CliError::runtime(format!("Failed to enable terminal raw mode: {e}")))?;
        io::stdout()
            .execute(cursor::Hide)
            .map_err(|e| CliError::runtime(format!("Failed to update terminal cursor: {e}")))?;
        Ok(Self)
    }
}

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = stdout.execute(cursor::Show);
        let _ = writeln!(stdout);
    }
}

fn draw_watch_frame(stdout: &mut io::Stdout, state: &Value) -> CliResult<()> {
    let phase = state
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let remaining = state
        .get("remainingSeconds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let next_phase = state
        .get("nextPhase")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = if state_is_running(state) {
        "Running"
    } else {
        "Paused"
    };

    write!(stdout, "\x1b[2J\x1b[H")
        .and_then(|_| writeln!(stdout, "Pomodoro Pulse watch mode"))
        .and_then(|_| writeln!(stdout, "Press q or Ctrl+C to exit.\n"))
        .and_then(|_| writeln!(stdout, "Current phase: {}\n", phase_label(phase)))
        .and_then(|_| writeln!(stdout, "\n{}\n", render_large_text(&format_time(remaining))))
        .and_then(|_| writeln!(stdout, "\nNext phase: {}", phase_label(next_phase)))
        .and_then(|_| writeln!(stdout, "Status: {status}"))
        .and_then(|_| stdout.flush())
        .map_err(|e| CliError::runtime(format!("Failed to draw watch view: {e}")))
}

fn run_watch(config: &Config) -> CliResult<()> {
    let mut stdout = io::stdout();
    let _guard = TerminalModeGuard::start()?;
    let mut next_refresh = Instant::now();

    loop {
        if Instant::now() >= next_refresh {
            let state = request_json(config, "GET", "/api/state")?;
            draw_watch_frame(&mut stdout, &state)?;
            next_refresh = Instant::now() + WATCH_REFRESH_INTERVAL;
        }

        if event::poll(WATCH_POLL_INTERVAL)
            .map_err(|e| CliError::runtime(format!("Failed to read keyboard input: {e}")))?
        {
            if let Event::Key(key) = event::read()
                .map_err(|e| CliError::runtime(format!("Failed to read keyboard event: {e}")))?
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                }
            }
        }
    }
    println!("Exited watch mode.");
    Ok(())
}

fn run_skip(config: &Config) -> CliResult<()> {
    request_json(config, "POST", "/api/skip")?;
    println!("Current phase skipped.");
    Ok(())
}

fn run_stop(config: &Config) -> CliResult<()> {
    request_json(config, "POST", "/api/pause")?;
    println!("Timer stopped.");
    Ok(())
}

fn run() -> CliResult<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = parse_args(&args)?;

    if command == Command::Help {
        print!("{HELP_TEXT}");
        return Ok(());
    }

    let path = config_path()?;
    match command {
        Command::Token(token) => {
            let mut config = load_config(&path)?;
            config.token = token;
            save_config(&path, &config)?;
            println!("Token saved.");
            Ok(())
        }
        Command::Port(port) => {
            let mut config = load_config(&path)?;
            config.port = port;
            save_config(&path, &config)?;
            println!("Port saved: {port}");
            Ok(())
        }
        Command::Status
        | Command::Watch
        | Command::Start
        | Command::Skip
        | Command::Stop
        | Command::Resume => {
            let config = load_config(&path)?;
            if config.token.trim().is_empty() {
                return Err(CliError::usage("Token is not set. Use: pp token <token>"));
            }
            match command {
                Command::Status => run_status(&config),
                Command::Watch => run_watch(&config),
                Command::Start => run_start(&config),
                Command::Resume => run_resume(&config),
                Command::Skip => run_skip(&config),
                Command::Stop => run_stop(&config),
                _ => Ok(()),
            }
        }
        Command::Help => Ok(()),
    }
}

fn main() {
    if let Err(error) = run() {
        if error.exit_code == 1 {
            eprintln!("{}\n\n{}", error.message, AVAILABLE_COMMANDS);
        } else {
            eprintln!("{}", error.message);
        }
        process::exit(error.exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decide_resume_action, decide_start_action, format_time, parse_args, render_large_text,
        Command, ResumeAction, StartAction,
    };
    use serde_json::json;

    fn vec_args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn parse_help_without_args() {
        let args: Vec<String> = Vec::new();
        assert_eq!(parse_args(&args).ok(), Some(Command::Help));
    }

    #[test]
    fn parse_token_and_port_commands() {
        assert_eq!(
            parse_args(&vec_args(&["token", "abc"])).ok(),
            Some(Command::Token("abc".to_string()))
        );
        assert_eq!(
            parse_args(&vec_args(&["port", "48484"])).ok(),
            Some(Command::Port(48484))
        );
    }

    #[test]
    fn parse_watch_and_resume_commands() {
        assert_eq!(parse_args(&vec_args(&["watch"])).ok(), Some(Command::Watch));
        assert_eq!(parse_args(&vec_args(&["live"])).ok(), Some(Command::Watch));
        assert_eq!(
            parse_args(&vec_args(&["resume"])).ok(),
            Some(Command::Resume)
        );
    }

    #[test]
    fn parse_rejects_unknown_command() {
        assert!(parse_args(&vec_args(&["unknown"])).is_err());
    }

    #[test]
    fn parse_rejects_invalid_port() {
        assert!(parse_args(&vec_args(&["port", "70000"])).is_err());
    }

    #[test]
    fn format_time_variants() {
        assert_eq!(format_time(125), "02:05");
        assert_eq!(format_time(3723), "1:02:03");
        assert_eq!(format_time(-10), "00:00");
    }

    #[test]
    fn decide_start_action_variants() {
        assert_eq!(
            decide_start_action(&json!({"isRunning": true, "startedAt": 123})),
            StartAction::AlreadyRunning
        );
        assert_eq!(
            decide_start_action(&json!({"isRunning": false, "startedAt": 123})),
            StartAction::ResumePaused
        );
        assert_eq!(
            decide_start_action(&json!({"isRunning": false, "startedAt": null})),
            StartAction::StartFresh
        );
    }

    #[test]
    fn decide_resume_action_variants() {
        assert_eq!(
            decide_resume_action(&json!({"isRunning": true, "startedAt": 123})),
            ResumeAction::AlreadyRunning
        );
        assert_eq!(
            decide_resume_action(&json!({"isRunning": false, "startedAt": 123})),
            ResumeAction::ResumePaused
        );
        assert_eq!(
            decide_resume_action(&json!({"isRunning": false, "startedAt": null})),
            ResumeAction::NoPausedSession
        );
    }

    #[test]
    fn render_large_text_shape() {
        let rendered = render_large_text("12:34");
        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|line| !line.is_empty()));
    }
}
