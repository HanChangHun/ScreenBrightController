#![cfg(windows)]
use screen_bright_controller::{session::Session, ui::UiSession};
#[test]
fn live_gesture_applies_without_consent_and_restore_rejects_old_generation() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert!(ui.status().unwrap()["armed"].as_array().unwrap().is_empty());
    ui.live_control("master", 40, true, 0).unwrap();
    assert_eq!(ui.status().unwrap()["operation"], "continuous");
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
    assert!(ui.status().unwrap()["armed"].as_array().unwrap().is_empty());
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
    assert_eq!(ui.status().unwrap()["armed"], serde_json::json!(["mock-2"]));
    ui.live_control("master", 0, true, 0).unwrap();
    assert!(ui.status().unwrap()["armed"].as_array().unwrap().is_empty());
}
#[test]
fn rejected_live_blocks_retry_preserves_protection_and_restore_failure_fences() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_set = true;
    }
    assert!(ui.live_control("master", 80, true, 0).is_err());
    assert!(!ui.status().unwrap()["armed"].as_array().unwrap().is_empty());
    assert_eq!(
        ui.status().unwrap()["outcomes"][0]["readback_matches"],
        false
    );
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
    assert!(ui.live_control("master", 80, true, 0).is_err());
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
            assert!(
                ui.live_control("master", dim, true, 0).is_err(),
                "dim {dim}"
            );
            let status = ui.status().unwrap();
            assert_eq!(status["outcomes"][0]["api_success"], !rejected);
            assert_eq!(status["outcomes"][0]["readback_matches"], false);
            assert!(status["message"].as_str().unwrap().contains("stopped"));
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
