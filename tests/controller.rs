use screen_bright_controller::controller::*;
#[test]
fn only_unplugged_display_failures_count_as_detached() {
    assert!(detached_only(DISCONNECTED));
    assert!(detached_only(&format!(
        "a: {DISCONNECTED}; b: {DISCONNECTED}"
    )));
    assert!(!detached_only(&format!(
        "a: {DISCONNECTED}; b: mock restore failure"
    )));
    assert!(!detached_only(""));
}
#[test]
fn arms_before_write_and_reports_readback() {
    let mut c = Controller::new(Mock::default()).unwrap();
    let result = c.apply_continuous(&["mock-1".into()], 75).unwrap();
    assert_eq!(c.driver.armed, vec!["mock-1"]);
    assert_eq!(c.driver.current["mock-1"][0][0], 30000);
    assert_eq!(result[0].readback_matches, Some(true));
    assert!(c.changed.contains("mock-1"));
}
#[test]
fn refuses_unknown_duplicate_or_unsafe_targets_before_any_write() {
    let mut c = Controller::new(Mock::default()).unwrap();
    assert!(c
        .apply_continuous(&["mock-1".into(), "missing".into()], 75)
        .is_err());
    assert!(c
        .apply_continuous(&["mock-1".into(), "mock-1".into()], 75)
        .is_err());
    assert!(c.apply_continuous(&["mock-1".into()], 9).is_err());
    assert_eq!(c.driver.writes, 0);
    assert!(c.driver.armed.is_empty());
}
#[test]
fn restore_only_changed_display_to_saved_original() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.writes, 0);
    c.apply_continuous(&["mock-1".into()], 50).unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.restored, vec!["mock-1"]);
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
    assert!(c.changed.is_empty());
    c.restore_all().unwrap();
    assert_eq!(c.driver.writes, 2);
}
#[test]
fn startup_snapshots_without_writing() {
    let c = Controller::new(Mock::default()).unwrap();
    assert_eq!(c.monitors.len(), 2);
    assert_eq!(c.driver.writes, 0);
    assert!(c.changed.is_empty());
}
