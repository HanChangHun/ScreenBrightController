#![cfg(windows)]
use screen_bright_controller::{linear_ramp, scale, session::Session, ui::UiSession};
use serde_json::json;

#[test]
fn safety_stop_and_failed_restore_expose_distinct_recovery_actions() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.current.get_mut("mock-1").unwrap()[0][0] = 123;
    }
    assert!(ui.heartbeat().is_err());
    let stopped = ui.status().unwrap();
    assert_eq!(stopped["live_blocked"], true);
    assert_eq!(stopped["recovery"], "safety");
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = true;
    }
    assert!(ui.restore().is_err());
    let failed = ui.status().unwrap();
    assert_eq!(failed["live_blocked"], true);
    assert_eq!(failed["recovery"], "restore");
    assert!(!failed["armed"].as_array().unwrap().is_empty());
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = false;
    }
    ui.restore().unwrap();
    assert_eq!(ui.status().unwrap()["recovery"], json!(null));
}

#[test]
fn failed_live_request_exposes_a_paused_state_until_explicit_restore() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert_eq!(ui.status().unwrap()["live_blocked"], false);
    assert_eq!(ui.status().unwrap()["recovery"], json!(null));
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_set = true;
    }
    assert!(ui.live_control("master", 52, true, 0).is_err());
    let failed = ui.status().unwrap();
    assert_eq!(failed["live_blocked"], true);
    assert_eq!(failed["recovery"], "apply");
    assert_eq!(failed["controls"]["master"]["dim"], 52); // A requested value, NOT an applied-value claim.
    assert_eq!(failed["outcomes"][0]["readback_matches"], false);
    assert!(ui.live_control("master", 53, true, 1).is_err());
    assert_eq!(ui.status().unwrap()["message"], failed["message"]);
    ui.heartbeat().unwrap();
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 3); // Two accepted writes, one ignored attempt; no retry loop.
        assert_eq!(c.driver.current["mock-1"][0][0], 26000);
    }
    ui.restore().unwrap();
    let restored = ui.status().unwrap();
    assert_eq!(restored["live_blocked"], false);
    assert_eq!(restored["recovery"], json!(null));
    assert_eq!(restored["controls"]["master"]["dim"], 0);
    assert_eq!(restored["armed"], json!([]));
}

#[test]
fn dimmed_baseline_is_flagged_and_reset_only_while_not_dimming() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert_eq!(ui.status().unwrap()["monitors"][0]["dimmed"], false);
    assert!(
        ui.reset_baseline().is_ok(),
        "nothing flagged, nothing written"
    );
    if let Session::Demo(c) = &mut ui.session {
        assert_eq!(c.driver.writes, 0);
        c.monitors[0].original = Some(scale(&linear_ramp(), 50).unwrap());
    }
    assert_eq!(ui.status().unwrap()["monitors"][0]["dimmed"], true);
    ui.live_control("master", 20, true, 0).unwrap();
    assert!(ui.reset_baseline().is_err());
    ui.restore().unwrap();
    ui.reset_baseline().unwrap();
    assert_eq!(ui.status().unwrap()["monitors"][0]["dimmed"], false);
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.current["mock-1"], linear_ramp());
    }
}
