#[cfg(not(windows))]
const CLI_INSTALL_MARKER_BEGIN: &str = "# >>> pomodoro-pulse pp >>>";
#[cfg(not(windows))]
const CLI_INSTALL_MARKER_END: &str = "# <<< pomodoro-pulse pp <<<";

fn pp_binary_filename() -> &'static str {
    if cfg!(windows) {
        "pp.exe"
    } else {
        "pp"
    }
}

fn resolve_bundled_pp_path(app: &AppHandle) -> AppResult<Option<std::path::PathBuf>> {
    let resource_dir = match app.path().resource_dir() {
        Ok(dir) => dir,
        Err(_) => return Ok(None),
    };

    let bundled = resource_dir.join("binaries").join(pp_binary_filename());
    if bundled.exists() {
        Ok(Some(bundled))
    } else {
        Ok(None)
    }
}

fn cli_install_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    Ok(home.join(".local").join("bin"))
}

fn install_pp_binary(source: &std::path::Path, target: &std::path::Path) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::copy(source, target).map_err(|e| {
        format!(
            "failed to install pp binary from {} to {}: {e}",
            source.display(),
            target.display()
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(target).map_err(|e| e.to_string())?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(target, permissions).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(windows)]
fn ensure_windows_user_path_contains(dir: &std::path::Path) -> AppResult<()> {
    use std::process::Command;

    let dir_value = dir.to_string_lossy().to_string();

    let output = Command::new("reg")
        .args(["query", "HKCU\\Environment", "/v", "Path"])
        .output()
        .map_err(|e| format!("failed to query user PATH: {e}"))?;

    let current = if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
                if tokens.len() >= 3
                    && tokens[0].eq_ignore_ascii_case("Path")
                    && tokens[1].starts_with("REG_")
                {
                    Some(tokens[2..].join(" "))
                } else {
                    None
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let normalize = |value: &str| -> String {
        value
            .trim()
            .trim_matches('"')
            .trim_end_matches('\\')
            .replace('/', "\\")
            .to_ascii_lowercase()
    };

    let current_entries = current
        .split(';')
        .map(normalize)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let wanted = normalize(&dir_value);
    if current_entries.iter().any(|entry| entry == &wanted) {
        return Ok(());
    }

    let next_value = if current.trim().is_empty() {
        dir_value.clone()
    } else {
        format!("{current};{dir_value}")
    };

    let status = Command::new("reg")
        .args([
            "add",
            "HKCU\\Environment",
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
            &next_value,
            "/f",
        ])
        .status()
        .map_err(|e| format!("failed to update user PATH: {e}"))?;

    if !status.success() {
        return Err("failed to update user PATH registry value".to_string());
    }

    Ok(())
}

#[cfg(not(windows))]
fn ensure_unix_shell_path_contains(app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        home.join(".bashrc")
    } else {
        home.join(".profile")
    };

    let mut content = fs::read_to_string(&rc_file).unwrap_or_default();
    if content.contains(CLI_INSTALL_MARKER_BEGIN) && content.contains(CLI_INSTALL_MARKER_END) {
        return Ok(());
    }

    let export_line = format!(
        "export PATH=\"{}:$PATH\"",
        install_dir.to_string_lossy().replace('"', "\\\"")
    );
    let block = format!(
        "{CLI_INSTALL_MARKER_BEGIN}\n{export_line}\n{CLI_INSTALL_MARKER_END}\n"
    );

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&block);

    fs::write(&rc_file, content).map_err(|e| {
        format!(
            "failed to update shell profile {}: {e}",
            rc_file.to_string_lossy()
        )
    })?;

    Ok(())
}

#[cfg(windows)]
fn ensure_cli_path_registration(_app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    ensure_windows_user_path_contains(install_dir)
}

#[cfg(not(windows))]
fn ensure_cli_path_registration(app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    ensure_unix_shell_path_contains(app, install_dir)
}

fn auto_install_pp_cli(app: &AppHandle) -> AppResult<()> {
    let Some(source) = resolve_bundled_pp_path(app)? else {
        // This may happen in development or if the installer did not include the CLI binary.
        return Ok(());
    };

    let install_dir = cli_install_dir(app)?;
    let target = install_dir.join(pp_binary_filename());

    install_pp_binary(&source, &target)?;
    ensure_cli_path_registration(app, &install_dir)?;
    Ok(())
}
