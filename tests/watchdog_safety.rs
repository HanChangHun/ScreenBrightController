use screen_bright_controller::{
    controller::Mock, guard::Guard, linear_ramp, scale, watchdog::Request,
};
#[test]
fn watchdog_protocol_never_accepts_external_ramp_or_snapshot_file() {
    for payload in [
        r#"{"command":"Continuous","id":"mock-1","lease":10,"snapshot":"x"}"#,
        r#"{"command":"Renew","id":"mock-1","lease":10,"mode":"preview"}"#,
        r#"{"command":"Renew","id":"mock-1","lease":1.5}"#,
    ] {
        assert!(serde_json::from_str::<Request>(payload).is_err());
    }
    assert!(serde_json::from_str::<Request>(
        r#"{"command":"Arm","id":"mock-1","seconds":15,"ramp":[1,2,3]}"#
    )
    .is_err());
    assert!(serde_json::from_str::<Request>(
        r#"{"command":"Restore","id":"mock-1","snapshot":"C:/arbitrary.json"}"#
    )
    .is_err());
    assert!(
        serde_json::from_str::<Request>(r#"{"command":"Reset","id":"mock-1","ramp":[1,2,3]}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<Request>(
        r#"{"command":"LoadSnapshot","path":"C:/arbitrary.json"}"#
    )
    .is_err());
}
#[test]
fn watchdog_rejects_unknown_restore_without_writes() {
    let mut g = Guard::new(Mock::default()).unwrap();
    assert!(g.restore("missing").is_err());
    g.restore("mock-1").unwrap();
    assert_eq!(g.driver.writes, 0);
}
#[test]
fn watchdog_rejects_unknown_duplicate_or_unbounded_arm() {
    let mut g = Guard::new(Mock::default()).unwrap();
    assert!(g.arm("missing", 15, 0).is_err());
    assert!(g.arm("mock-1", 0, 0).is_err());
    assert!(g.arm("mock-1", 31, 0).is_err());
    g.arm("mock-1", 15, 0).unwrap();
    assert!(g.arm("mock-1", 15, 1).is_err());
    assert_eq!(g.driver.writes, 0);
}
#[test]
fn watchdog_retains_owned_snapshot_after_restore_failure_and_retries() {
    let mut g = Guard::new(Mock {
        fail_restore: true,
        ..Default::default()
    })
    .unwrap();
    g.arm("mock-1", 1, 0).unwrap();
    g.tick(1000, true);
    assert!(g.armed.contains_key("mock-1"));
    assert!(!g.errors.is_empty());
    g.driver.fail_restore = false;
    g.tick(2000, true);
    assert!(g.armed.is_empty());
    assert_eq!(g.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn dimmed_baseline_resets_to_linear_only_on_request_and_while_unarmed() {
    let mut g = Guard::new(Mock::default()).unwrap();
    assert!(
        g.reset("mock-1").is_err(),
        "normal originals are never reset"
    );
    assert_eq!(g.driver.writes, 0);
    let dimmed = scale(&linear_ramp(), 50).unwrap();
    g.monitors[0].original = Some(dimmed.clone());
    g.arm_continuous("mock-1", 10, 0).unwrap();
    assert!(g.reset("mock-1").is_err());
    g.restore("mock-1").unwrap();
    assert_eq!(g.driver.current["mock-1"], dimmed);
    g.reset("mock-1").unwrap();
    assert_eq!(g.monitors[0].original, Some(linear_ramp()));
    assert_eq!(g.driver.current["mock-1"], linear_ramp());
}
