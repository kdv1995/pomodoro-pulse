use serde_json::{json, Value};
use std::{
    env, fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process,
    time::Duration,
};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 48484;
const CONFIG_FILE_NAME: &str = ".pomodoro-pulse-pp.json";

const HELP_TEXT: &str = r#"pp - Pomodoro Pulse HTTP CLI

Usage:
  pp <subcommand>
  pp help

Commands:
  status         Current phase, time to end, next phase
  start          Start/Resume current phase
  skip           Skip current phase
  stop           Stop timer
  token <token>  Save token for next commands
  port <port>    Save port for next commands
  help           Show this help

Config:
  token and port are stored in ~/.pomodoro-pulse-pp.json
  host is fixed to 127.0.0.1
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
    Start,
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
        "start" => {
            if args.len() > 1 {
                return Err(CliError::usage(format!("Unexpected argument: {}", args[1])));
            }
            Ok(Command::Start)
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
    if state
        .get("isRunning")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        println!("Timer is already running.");
        return Ok(());
    }

    let started = !state.get("startedAt").map(|v| v.is_null()).unwrap_or(true);
    if started {
        request_json(config, "POST", "/api/resume")?;
        println!("Timer resumed.");
    } else {
        request_json(config, "POST", "/api/start")?;
        println!("Timer started.");
    }
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
        Command::Status | Command::Start | Command::Skip | Command::Stop => {
            let config = load_config(&path)?;
            if config.token.trim().is_empty() {
                return Err(CliError::usage("Token is not set. Use: pp token <token>"));
            }
            match command {
                Command::Status => run_status(&config),
                Command::Start => run_start(&config),
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
            eprintln!("{}\n\n{}", error.message, HELP_TEXT);
        } else {
            eprintln!("{}", error.message);
        }
        process::exit(error.exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::{format_time, parse_args, Command};

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
}
