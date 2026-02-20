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

