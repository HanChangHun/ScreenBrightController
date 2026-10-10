#![cfg(windows)]
use screen_bright_controller::{
    linear_ramp, scale,
    session::Session,
    ui::{Recovery, UiSession},
};
fn armed(ui: &UiSession) -> Vec<String> {
    let Session::Demo(c) = &ui.session else {
        unreachable!()
    };
    c.changed.iter().cloned().collect()
}

#[test]
fn safety_stop_and_failed_restore_expose_distinct_recovery_actions() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.current.get_mut("mock-1").unwrap()[0][0] = 123;
    }
    assert!(ui.heartbeat().is_err());
    let stopped = ui.status().unwrap();
    assert!(stopped.live_blocked);
    assert_eq!(stopped.recovery, Some(Recovery::Safety));
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = true;
    }
    assert!(ui.restore().is_err());
    let failed = ui.status().unwrap();
    assert!(failed.live_blocked);
    assert_eq!(failed.recovery, Some(Recovery::Restore));
    assert!(!armed(&ui).is_empty());
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = false;
    }
    ui.restore().unwrap();
    assert_eq!(ui.status().unwrap().recovery, None);
}

#[test]
fn failed_live_request_exposes_a_paused_state_until_explicit_restore() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert!(!ui.status().unwrap().live_blocked);
    assert_eq!(ui.status().unwrap().recovery, None);
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_set = true;
    }
    assert!(ui.live_control("master", 52, true, 0).is_err());
    let failed = ui.status().unwrap();
    assert!(failed.live_blocked);
    assert_eq!(failed.recovery, Some(Recovery::Apply));
    assert_eq!(failed.controls["master"].dim, 52); // A requested value, NOT an applied-value claim.
    assert_eq!(failed.outcomes[0].readback_matches, Some(false));
    assert!(ui.live_control("master", 53, true, 1).is_err());
    assert_eq!(ui.status().unwrap().message, failed.message);
    ui.heartbeat().unwrap();
    assert!(
        ui.needs_attention(),
        "a healthy heartbeat cannot clear recovery"
    );
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 3); // Two accepted writes, one ignored attempt; no retry loop.
        assert_eq!(c.driver.current["mock-1"][0][0], 26000);
    }
    ui.restore().unwrap();
    let restored = ui.status().unwrap();
    assert!(!restored.live_blocked);
    assert_eq!(restored.recovery, None);
    assert_eq!(restored.controls["master"].dim, 0);
    assert!(armed(&ui).is_empty());
}

#[test]
fn dimmed_baseline_is_flagged_and_reset_only_while_not_dimming() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    assert!(!ui.status().unwrap().monitors[0].dimmed);
    assert!(
        ui.reset_baseline().is_ok(),
        "nothing flagged, nothing written"
    );
    if let Session::Demo(c) = &mut ui.session {
        assert_eq!(c.driver.writes, 0);
        c.monitors[0].original = Some(scale(&linear_ramp(), 50).unwrap());
    }
    assert!(ui.status().unwrap().monitors[0].dimmed);
    ui.live_control("master", 20, true, 0).unwrap();
    assert!(ui.reset_baseline().is_err());
    ui.restore().unwrap();
    ui.reset_baseline().unwrap();
    assert!(!ui.status().unwrap().monitors[0].dimmed);
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.current["mock-1"], linear_ramp());
    }
}
