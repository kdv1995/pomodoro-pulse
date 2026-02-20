fn remote_stop(remote: &mut RemoteControlState) {
    if let Some(mut handle) = remote.server.take() {
        handle.stop.store(false, Ordering::SeqCst);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }
}

fn bind_remote_listener(port: u16) -> AppResult<TcpListener> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("remote control server bind failed on {addr}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("remote control server nonblocking setup failed on {addr}: {e}"))?;
    Ok(listener)
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

    let listener = bind_remote_listener(port)?;
    let stop = Arc::new(AtomicBool::new(true));
    let stop_thread = stop.clone();
    let app_handle = app.clone();

    let join = thread::spawn(move || remote_server_loop(app_handle, listener, stop_thread));
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

const REMOTE_CSP: &str = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";

fn write_response(stream: &mut std::net::TcpStream, code: &str, content_type: &str, body: &[u8]) {
    let headers = format!(
        "HTTP/1.1 {code}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type, X-Pomodoro-Token\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Security-Policy: {REMOTE_CSP}\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nPermissions-Policy: geolocation=(), microphone=(), camera=()\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body);
}

fn predict_next_phase(timer: &TimerState, settings: &AppSettings) -> TimerPhase {
    match timer.phase {
        TimerPhase::Focus => {
            let next_cycle_index = timer.cycle_index + 1;
            if next_cycle_index % settings.long_break_every == 0 {
                TimerPhase::LongBreak
            } else {
                TimerPhase::ShortBreak
            }
        }
        TimerPhase::ShortBreak | TimerPhase::LongBreak => TimerPhase::Focus,
    }
}

fn remote_state_payload(state: &AppState) -> AppResult<serde_json::Value> {
    let mut model = state.model.lock().map_err(|e| e.to_string())?;
    refresh_remaining(&mut model.timer);

    let timer = model.timer.clone();
    let next_phase = predict_next_phase(&timer, &model.settings);

    let mut payload = serde_json::to_value(&timer).map_err(|e| e.to_string())?;
    match &mut payload {
        serde_json::Value::Object(map) => {
            map.insert(
                "nextPhase".to_string(),
                serde_json::to_value(next_phase).map_err(|e| e.to_string())?,
            );
            Ok(payload)
        }
        _ => Err("failed to build remote timer state payload".to_string()),
    }
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
          <p class="muted">Enter your token (from the desktop app Settings) to control the timer.</p>
          <input id="token" class="token" placeholder="Token" autocomplete="off" />
          <div class="sp"></div>
          <button class="primary" id="saveToken">Continue</button>
        </div>
        <div id="main" style="display:none">
          <div class="row">
            <div>
              <div class="muted">
                <span id="phase">...</span>
                <span>&nbsp;&rarr;&nbsp;</span>
                <span>Next:&nbsp;</span>
                <span id="next_phase">...</span>
              </div>
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
      <p class="muted">If this page does not load: ensure Remote Control is enabled in the desktop app Settings, and your phone and desktop are on the same Wi‑Fi.</p>
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
          document.getElementById("next_phase").textContent = phaseLabel(st.nextPhase);
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
        ("GET", "/api/state") => match remote_state_payload(state.inner()) {
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

fn remote_server_loop(app: AppHandle, listener: TcpListener, stop: Arc<AtomicBool>) {
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

