#[cfg(not(windows))]
const CLI_INSTALL_MARKER_BEGIN: &str = "# >>> pomodoro-pulse pp >>>";
#[cfg(not(windows))]
const CLI_INSTALL_MARKER_END: &str = "# <<< pomodoro-pulse pp <<<";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PpCliStatus {
    binary_installed: bool,
    path_configured: bool,
    install_dir: String,
    binary_path: String,
}

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

fn cli_binary_path(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    Ok(cli_install_dir(app)?.join(pp_binary_filename()))
}

fn status_from(binary_path: &std::path::Path, install_dir: &std::path::Path, path_configured: bool) -> PpCliStatus {
    PpCliStatus {
        binary_installed: binary_path.is_file(),
        path_configured,
        install_dir: install_dir.to_string_lossy().to_string(),
        binary_path: binary_path.to_string_lossy().to_string(),
    }
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
fn normalize_windows_path(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_end_matches('\\')
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(windows)]
fn split_windows_path_entries(path_value: &str) -> Vec<String> {
    path_value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect::<Vec<_>>()
}

#[cfg(windows)]
fn windows_path_contains_entry(path_value: &str, dir: &std::path::Path) -> bool {
    let wanted = normalize_windows_path(&dir.to_string_lossy());
    split_windows_path_entries(path_value)
        .iter()
        .any(|entry| normalize_windows_path(entry) == wanted)
}

#[cfg(windows)]
fn windows_path_without_entry(path_value: &str, dir: &std::path::Path) -> (String, bool) {
    let wanted = normalize_windows_path(&dir.to_string_lossy());
    let entries = split_windows_path_entries(path_value);
    let retained = entries
        .iter()
        .filter(|entry| normalize_windows_path(entry) != wanted)
        .cloned()
        .collect::<Vec<_>>();
    let changed = retained.len() != entries.len();
    (retained.join(";"), changed)
}

#[cfg(windows)]
fn current_windows_user_path() -> AppResult<String> {
    use std::process::Command;

    let output = Command::new("reg")
        .args(["query", "HKCU\\Environment", "/v", "Path"])
        .output()
        .map_err(|e| format!("failed to query user PATH: {e}"))?;

    if !output.status.success() {
        return Ok(String::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
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
        .unwrap_or_default())
}

#[cfg(windows)]
fn set_windows_user_path(next_value: &str) -> AppResult<()> {
    use std::process::Command;

    let status = Command::new("reg")
        .args([
            "add",
            "HKCU\\Environment",
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
            next_value,
            "/f",
        ])
        .status()
        .map_err(|e| format!("failed to update user PATH: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("failed to update user PATH registry value".to_string())
    }
}

#[cfg(windows)]
fn ensure_windows_user_path_contains(dir: &std::path::Path) -> AppResult<()> {
    let current = current_windows_user_path()?;
    if windows_path_contains_entry(&current, dir) {
        return Ok(());
    }

    let mut entries = split_windows_path_entries(&current);
    entries.push(dir.to_string_lossy().to_string());
    set_windows_user_path(&entries.join(";"))
}

#[cfg(windows)]
fn is_windows_user_path_contains(dir: &std::path::Path) -> AppResult<bool> {
    let current = current_windows_user_path()?;
    Ok(windows_path_contains_entry(&current, dir))
}

#[cfg(windows)]
fn remove_windows_user_path_entry(dir: &std::path::Path) -> AppResult<()> {
    let current = current_windows_user_path()?;
    let (next, changed) = windows_path_without_entry(&current, dir);
    if !changed {
        return Ok(());
    }
    set_windows_user_path(&next)
}

#[cfg(not(windows))]
fn shell_rc_file(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        home.join(".bashrc")
    } else {
        home.join(".profile")
    };
    Ok(rc_file)
}

#[cfg(not(windows))]
fn has_managed_unix_path_block(content: &str) -> bool {
    content.contains(CLI_INSTALL_MARKER_BEGIN) && content.contains(CLI_INSTALL_MARKER_END)
}

#[cfg(not(windows))]
fn strip_managed_unix_path_block(content: &str) -> (String, bool) {
    let Some(start_idx) = content.find(CLI_INSTALL_MARKER_BEGIN) else {
        return (content.to_string(), false);
    };

    let end_search_start = start_idx + CLI_INSTALL_MARKER_BEGIN.len();
    let Some(end_rel_idx) = content[end_search_start..].find(CLI_INSTALL_MARKER_END) else {
        return (content.to_string(), false);
    };
    let end_idx = end_search_start + end_rel_idx + CLI_INSTALL_MARKER_END.len();

    let mut remove_end = end_idx;
    while remove_end < content.len() && &content[remove_end..remove_end + 1] == "\n" {
        remove_end += 1;
    }

    let mut updated = String::with_capacity(content.len());
    updated.push_str(&content[..start_idx]);
    updated.push_str(&content[remove_end..]);
    (updated, true)
}

#[cfg(not(windows))]
fn ensure_unix_shell_path_contains(app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    let rc_file = shell_rc_file(app)?;
    let mut content = fs::read_to_string(&rc_file).unwrap_or_default();
    if has_managed_unix_path_block(&content) {
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

#[cfg(not(windows))]
fn is_unix_shell_path_registered(app: &AppHandle) -> AppResult<bool> {
    let rc_file = shell_rc_file(app)?;
    let content = fs::read_to_string(&rc_file).unwrap_or_default();
    Ok(has_managed_unix_path_block(&content))
}

#[cfg(not(windows))]
fn remove_unix_shell_path_registration(app: &AppHandle) -> AppResult<()> {
    let rc_file = shell_rc_file(app)?;
    let content = fs::read_to_string(&rc_file).unwrap_or_default();
    let (next, changed) = strip_managed_unix_path_block(&content);
    if !changed {
        return Ok(());
    }

    fs::write(&rc_file, next).map_err(|e| {
        format!(
            "failed to update shell profile {}: {e}",
            rc_file.to_string_lossy()
        )
    })
}

#[cfg(windows)]
fn ensure_cli_path_registration(_app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    ensure_windows_user_path_contains(install_dir)
}

#[cfg(not(windows))]
fn ensure_cli_path_registration(app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    ensure_unix_shell_path_contains(app, install_dir)
}

#[cfg(windows)]
fn is_cli_path_registered(_app: &AppHandle, install_dir: &std::path::Path) -> AppResult<bool> {
    is_windows_user_path_contains(install_dir)
}

#[cfg(not(windows))]
fn is_cli_path_registered(app: &AppHandle, _install_dir: &std::path::Path) -> AppResult<bool> {
    is_unix_shell_path_registered(app)
}

#[cfg(windows)]
fn remove_cli_path_registration(_app: &AppHandle, install_dir: &std::path::Path) -> AppResult<()> {
    remove_windows_user_path_entry(install_dir)
}

#[cfg(not(windows))]
fn remove_cli_path_registration(app: &AppHandle, _install_dir: &std::path::Path) -> AppResult<()> {
    remove_unix_shell_path_registration(app)
}

fn pp_cli_status_inner(app: &AppHandle) -> AppResult<PpCliStatus> {
    let install_dir = cli_install_dir(app)?;
    let binary_path = cli_binary_path(app)?;
    let path_configured = is_cli_path_registered(app, &install_dir)?;
    Ok(status_from(&binary_path, &install_dir, path_configured))
}

fn pp_cli_uninstall_inner(app: &AppHandle) -> AppResult<PpCliStatus> {
    let install_dir = cli_install_dir(app)?;
    let binary_path = cli_binary_path(app)?;

    match fs::remove_file(&binary_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to remove pp binary {}: {error}",
                binary_path.display()
            ));
        }
    }

    remove_cli_path_registration(app, &install_dir)?;
    let path_configured = is_cli_path_registered(app, &install_dir)?;
    Ok(status_from(&binary_path, &install_dir, path_configured))
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

#[cfg(test)]
mod cli_installer_tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn unix_strip_managed_path_block_removes_only_managed_lines() {
        let content = format!(
            "line-a\n{CLI_INSTALL_MARKER_BEGIN}\nexport PATH=\"/tmp/bin:$PATH\"\n{CLI_INSTALL_MARKER_END}\nline-b\n"
        );
        let (next, changed) = strip_managed_unix_path_block(&content);
        assert!(changed);
        assert_eq!(next, "line-a\nline-b\n");
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_strip_managed_path_block_is_idempotent_without_markers() {
        let content = "line-a\nline-b\n";
        let (next, changed) = strip_managed_unix_path_block(content);
        assert!(!changed);
        assert_eq!(next, content);
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_without_entry_removes_only_target_value() {
        let current = r"C:\tools;C:\Users\me\.local\bin;C:\other";
        let target = std::path::Path::new(r"C:\Users\me\.local\bin\");
        let (next, changed) = windows_path_without_entry(current, target);
        assert!(changed);
        assert_eq!(next, r"C:\tools;C:\other");
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_without_entry_leaves_path_unchanged_when_missing() {
        let current = r"C:\tools;C:\other";
        let target = std::path::Path::new(r"C:\Users\me\.local\bin");
        let (next, changed) = windows_path_without_entry(current, target);
        assert!(!changed);
        assert_eq!(next, current);
    }

    #[test]
    fn status_from_reports_binary_and_path_flags() {
        let install_dir = std::path::PathBuf::from("C:/tmp/bin");
        let binary_path = install_dir.join(pp_binary_filename());

        let status_missing = status_from(&binary_path, &install_dir, false);
        assert!(!status_missing.binary_installed);
        assert!(!status_missing.path_configured);

        let temp_root = std::env::temp_dir().join(format!(
            "pp-cli-status-test-{}-{}",
            std::process::id(),
            now_ts()
        ));
        let test_binary = temp_root.join(pp_binary_filename());
        fs::create_dir_all(&temp_root).expect("failed to create temp directory");
        fs::write(&test_binary, b"pp").expect("failed to create binary file");

        let status_present = status_from(&test_binary, &temp_root, true);
        assert!(status_present.binary_installed);
        assert!(status_present.path_configured);

        let _ = fs::remove_file(test_binary);
        let _ = fs::remove_dir_all(temp_root);
    }
}
