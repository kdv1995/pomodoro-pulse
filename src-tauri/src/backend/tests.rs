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

#[test]
fn remote_listener_bind_succeeds_on_available_port() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("failed to reserve probe port");
    let port = probe
        .local_addr()
        .expect("failed to get probe local addr")
        .port();
    drop(probe);

    let listener = bind_remote_listener(port).expect("expected bind to succeed");
    let _ = listener
        .local_addr()
        .expect("listener should have local addr");
}

#[test]
fn remote_listener_bind_returns_error_when_port_is_occupied() {
    let occupied = TcpListener::bind("0.0.0.0:0").expect("failed to reserve an occupied test port");
    let port = occupied
        .local_addr()
        .expect("failed to get occupied local addr")
        .port();

    let err = bind_remote_listener(port).expect_err("expected occupied port bind failure");
    assert!(err.contains("bind failed"));
}
