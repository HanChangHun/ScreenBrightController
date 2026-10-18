#![cfg(windows)]
use screen_bright_controller::{session::Session, ui::UiSession};
#[test]
fn tray_wheel_matches_gui_up_darkens_down_brightens_and_keeps_exclusions() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.tray_wheel(1, 0).unwrap();
    assert_eq!(ui.controls["master"].dim, 5);
    ui.live_control("mock-2", 0, false, ui.generation).unwrap();
    ui.tray_wheel(2, ui.generation).unwrap();
    assert_eq!(ui.controls["master"].dim, 15);
    assert_eq!(armed(&ui), ["mock-1"]);
    ui.tray_wheel(-1, ui.generation).unwrap();
    assert_eq!(ui.controls["master"].dim, 10);
    ui.tray_wheel(i32::MAX, ui.generation).unwrap();
    assert_eq!(ui.controls["master"].dim, 90);
    ui.tray_wheel(i32::MIN, ui.generation).unwrap();
    assert_eq!(ui.controls["master"].dim, 0);
    assert!(armed(&ui).is_empty());
    let revision = ui.revision;
    ui.tray_wheel(-1, ui.generation).unwrap();
    ui.tray_wheel(0, ui.generation).unwrap();
    assert_eq!(ui.revision, revision);
}
#[test]
fn tray_wheel_preserves_unlinked_mode_and_cancels_old_input_after_restore() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 0, false, 0).unwrap();
    ui.live_control("mock-1", 23, true, 0).unwrap();
    ui.tray_wheel(1, ui.generation).unwrap();
    assert!(!ui.controls["master"].enabled);
    assert_eq!(ui.controls["mock-1"].dim, 23);
    ui.restore().unwrap();
    assert!(ui.tray_wheel(1, 0).is_err());
    assert!(armed(&ui).is_empty());
    ui.live_control("master", 0, true, ui.generation).unwrap();
    let old = ui.generation;
    ui.tray_wheel(1, old).unwrap();
    assert!(
        ui.generation > old,
        "tray gesture fences pending popup input"
    );
    assert!(ui.live_control("master", 60, true, old).is_err());
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_set = true;
    }
    assert!(ui.tray_wheel(1, ui.generation).is_err());
    assert!(ui.tray_wheel(1, ui.generation).is_err());
}
#[test]
fn tray_wheel_filters_injected_outside_stale_and_accumulates_high_resolution() {
    use screen_bright_controller::ui::TrayWheel;
    let mut wheel = TrayWheel::default();
    assert_eq!(wheel.steps(60, true, false, 0, 0), 0);
    assert_eq!(wheel.steps(60, true, false, 0, 1), 1);
    assert_eq!(wheel.steps(-240, true, false, 0, 2), -2);
    assert_eq!(wheel.steps(120, true, true, 0, 3), 0);
    assert_eq!(wheel.steps(120, false, false, 0, 4), 0);
    assert_eq!(wheel.steps(60, true, false, 0, 5), 0);
    assert_eq!(wheel.steps(60, true, false, 1, 6), 0);
    assert_eq!(wheel.steps(60, true, false, 1, 1000), 0);
    assert_eq!(wheel.steps(120, true, false, 1, 1001), 1);
}
fn armed(ui: &UiSession) -> Vec<String> {
    let Session::Demo(c) = &ui.session else {
        unreachable!()
    };
    c.changed.iter().cloned().collect()
}
#[test]
fn live_gesture_applies_without_consent_and_restore_rejects_old_generation() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert!(armed(&ui).is_empty());
    ui.live_control("master", 40, true, 0).unwrap();
    assert!(!armed(&ui).is_empty());
    ui.live_control("master", 65, true, 0).unwrap();
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.armed.len(), 2);
        assert_eq!(c.driver.current["mock-1"][0][0], 14000);
    }
    ui.live_control("master", 23, true, 0).unwrap();
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.current["mock-1"][0][0], 30800);
    }
    ui.restore().unwrap();
    assert!(ui.live_control("master", 50, true, 0).is_err());
    // An obsolete pre-Restore request is cancelled, not a new recovery state.
    assert!(!ui.status().unwrap().live_blocked);
    assert!(!ui.needs_attention());
    assert!(armed(&ui).is_empty());
}
#[test]
fn live_links_independent_exclusion_zero_and_readonly_status() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    for _ in 0..3 {
        ui.status().unwrap();
        ui.heartbeat().unwrap();
    }
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 0);
    }
    for dim in [-1, 91, i64::MAX] {
        assert!(ui.live_control("master", dim, true, 0).is_err());
    }
    assert!(ui.live_control("missing", 23, true, 0).is_err());
    ui.live_control("master", 0, false, 0).unwrap();
    ui.live_control("mock-1", 23, true, 0).unwrap();
    ui.live_control("mock-2", 42, true, 0).unwrap();
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 2);
    }
    ui.live_control("master", 33, true, 0).unwrap();
    ui.live_control("mock-1", 23, false, 0).unwrap();
    assert_eq!(armed(&ui), ["mock-2"]);
    ui.live_control("master", 0, true, 0).unwrap();
    assert!(armed(&ui).is_empty());
}
#[test]
fn unverified_live_blocks_retry_preserves_protection_and_restore_failure_fences() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        // The API reports failure although the ramp changed: the state is not verified.
        c.driver.fail_set = true;
    }
    assert!(ui.live_control("master", 80, true, 0).is_err());
    assert!(!armed(&ui).is_empty());
    let status = ui.status().unwrap();
    assert!(!status.outcomes[0].api_success);
    assert!(!status.outcomes[0].kept_previous);
    assert!(ui.live_control("master", 81, true, 1).is_err());
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = true;
    }
    assert!(ui.restore().is_err());
    assert!(ui.live_control("master", 10, true, 2).is_err());
}
#[test]
fn ignored_live_update_keeps_last_verified_ramp_without_fighting() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 40, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_set = true;
    }
    ui.live_control("master", 80, true, 0).unwrap();
    assert_eq!(ui.status().unwrap().controls["master"].dim, 40);
    ui.heartbeat().unwrap();
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.current["mock-1"][0][0], 24000);
        assert!(c.changed.contains("mock-1"));
        assert_eq!(c.driver.writes, 3);
    }
}

#[test]
fn boundary_requests_report_ignored_and_rejected_ramps_honestly() {
    for dim in [44, 45, 46, 49, 50, 51, 55, 90] {
        for rejected in [false, true] {
            let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
            ui.live_control("master", 35, true, 0).unwrap();
            if let Session::Demo(c) = &mut ui.session {
                c.driver.ignored_set = true;
                c.driver.fail_set = rejected;
            }
            ui.live_control("master", dim, true, 0).unwrap();
            let status = ui.status().unwrap();
            assert_eq!(status.outcomes[0].api_success, !rejected);
            assert_eq!(status.outcomes[0].readback_matches, Some(false));
            assert!(status.outcomes[0].kept_previous);
            assert_eq!(
                status.notice.as_deref(),
                Some(format!("Windows rejected {dim} and kept 35.").as_str())
            );
            assert!(!status.live_blocked && status.recovery.is_none());
            ui.heartbeat().unwrap();
            if let Session::Demo(c) = &ui.session {
                assert_eq!(c.driver.current["mock-1"][0][0], 26000);
                assert_eq!(c.driver.writes, 3);
            }
            ui.restore().unwrap();
            if let Session::Demo(c) = &ui.session {
                assert_eq!(c.driver.current["mock-1"][0][0], 40000);
                assert_eq!(c.driver.current["mock-2"][0][0], 50000);
            }
        }
    }
}
