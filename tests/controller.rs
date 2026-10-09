use screen_bright_controller::controller::*;
#[test]
fn deliberate_preview_arms_before_write_and_reports_readback() {
    let mut c = Controller::new(Mock::default()).unwrap();
    let result = c.preview(&["mock-1".into()], 75, 15, true).unwrap();
    assert_eq!(c.driver.armed, vec!["mock-1"]);
    assert_eq!(c.driver.current["mock-1"][0][0], 30000);
    assert_eq!(result[0].readback_matches, Some(true));
    assert!(!result[0].visible_effect_verified);
    assert!(c.changed.contains_key("mock-1"));
}
#[test]
fn refuses_without_consent_invalid_timer_or_rearming() {
    let mut c = Controller::new(Mock::default()).unwrap();
    assert!(c.preview(&["mock-1".into()], 75, 15, false).is_err());
    assert!(c.preview(&["mock-1".into()], 75, 0, true).is_err());
    assert!(c.preview(&["mock-1".into()], 75, 31, true).is_err());
    assert!(c
        .preview(&["mock-1".into(), "missing".into()], 75, 15, true)
        .is_err());
    assert_eq!(c.driver.writes, 0);
    c.preview(&["mock-1".into()], 75, 15, true).unwrap();
    assert!(c.preview(&["mock-1".into()], 60, 15, true).is_err());
    assert_eq!(c.driver.writes, 1);
}
#[test]
fn restore_only_changed_display_to_saved_original() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.writes, 0);
    c.preview(&["mock-1".into()], 50, 15, true).unwrap();
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
