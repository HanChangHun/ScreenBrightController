#![cfg(windows)]
use screen_bright_controller::{
    controller::DISCONNECTED,
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
        c.driver.fail_set = true;
    }
    assert!(ui.live_control("master", 52, true, 0).is_err());
    let failed = ui.status().unwrap();
    assert!(failed.live_blocked);
    assert_eq!(failed.recovery, Some(Recovery::Apply));
    assert_eq!(failed.controls["master"].dim, 52); // A requested value, NOT an applied-value claim.
    assert!(!failed.outcomes[0].api_success);
    assert!(ui.live_control("master", 53, true, 1).is_err());
    assert_eq!(ui.status().unwrap().message, failed.message);
    ui.heartbeat().unwrap();
    assert!(
        ui.needs_attention(),
        "a healthy heartbeat cannot clear recovery"
    );
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 3); // Two accepted writes, one unverified attempt; no retry loop.
        assert_eq!(c.driver.current["mock-1"][0][0], 19200);
    }
    ui.restore().unwrap();
    let restored = ui.status().unwrap();
    assert!(!restored.live_blocked);
    assert_eq!(restored.recovery, None);
    assert_eq!(restored.controls["master"].dim, 0);
    assert!(armed(&ui).is_empty());
}

#[test]
fn rejected_update_keeps_the_verified_level_with_a_notice_instead_of_a_lock() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_ids = vec!["mock-2".into()];
    }
    // Display 1 accepts 52, display 2 keeps 35: display 1 returns to 35 with the controls.
    ui.live_control("master", 52, true, 0).unwrap();
    let kept = ui.status().unwrap();
    assert!(!kept.live_blocked && kept.recovery.is_none() && !ui.needs_attention());
    assert_eq!(kept.controls["master"].dim, 35);
    assert_eq!(kept.generation, 1, "requests queued on 52 are cancelled");
    assert_eq!(
        kept.notice.as_deref(),
        Some("Windows rejected 52 and kept 35.")
    );
    assert!(kept.outcomes[1].kept_previous && kept.outcomes[1].readback_matches == Some(false));
    if let Session::Demo(c) = &ui.session {
        assert_eq!(c.driver.writes, 5); // 2 accepted, 1 accepted + 1 ignored, 1 return to 35.
        assert_eq!(c.driver.current["mock-1"][0][0], 26000);
        assert_eq!(c.driver.current["mock-2"][0][0], 32500);
    }
    assert!(ui.live_control("master", 40, true, 0).is_err());
    assert!(!ui.status().unwrap().live_blocked);
    if let Session::Demo(c) = &mut ui.session {
        c.driver.ignored_ids.clear();
    }
    ui.live_control("master", 45, true, 1).unwrap();
    let next = ui.status().unwrap();
    assert_eq!(next.notice, None);
    assert_eq!(next.controls["master"].dim, 45);
    // A first write that is refused leaves the display unarmed at its original.
    let mut fresh = UiSession::new(Session::demo().unwrap()).unwrap();
    if let Session::Demo(c) = &mut fresh.session {
        c.driver.ignored_set = true;
    }
    fresh.live_control("master", 60, true, 0).unwrap();
    assert!(armed(&fresh).is_empty());
    assert_eq!(fresh.status().unwrap().controls["master"].dim, 0);
    if let Session::Demo(c) = &fresh.session {
        assert_eq!(c.driver.current["mock-1"][0][0], 40000);
    }
}

#[test]
fn unplugged_display_offers_quit_anyway_and_restores_once_it_returns() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.live_control("master", 35, true, 0).unwrap();
    if let Session::Demo(c) = &mut ui.session {
        c.driver.detached = vec!["mock-2".into()];
    }
    assert_eq!(ui.heartbeat().unwrap_err(), DISCONNECTED);
    assert!(ui.detached());
    let error = ui.restore().unwrap_err();
    assert_eq!(error, format!("mock-2: {DISCONNECTED}"));
    let status = ui.status().unwrap();
    assert_eq!(status.recovery, Some(Recovery::Detached));
    assert!(status.live_blocked && ui.detached());
    assert_eq!(armed(&ui), ["mock-2"]);
    if let Session::Demo(c) = &ui.session {
        assert_eq!(
            c.driver.current["mock-1"][0][0], 40000,
            "attached displays restore"
        );
    }
    // Any other failure keeps the ordinary Restore failure: Quit stays cancelled.
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = true;
        c.driver.detached.clear();
    }
    assert!(ui.restore().is_err());
    assert!(!ui.detached());
    if let Session::Demo(c) = &mut ui.session {
        c.driver.fail_restore = false;
    }
    ui.restore().unwrap();
    assert!(!ui.detached() && !ui.needs_attention());
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
