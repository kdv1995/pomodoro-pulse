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
                if let Err(error) = remote_apply(app.handle(), &model.settings) {
                    eprintln!("remote control startup warning: {error}");
                }
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

