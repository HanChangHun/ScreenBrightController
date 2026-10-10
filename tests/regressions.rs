use screen_bright_controller::controller::*;
#[test]
fn restoration_failure_preserves_original_and_allows_retry() {
    let mut c = Controller::new(Mock {
        fail_restore: true,
        ..Default::default()
    })
    .unwrap();
    c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(c.restore_all().is_err());
    assert!(c.changed.contains("mock-1"));
    c.driver.fail_restore = false;
    c.restore_all().unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn watchdog_failure_prevents_display_write() {
    let mut c = Controller::new(Mock {
        fail_arm: true,
        ..Default::default()
    })
    .unwrap();
    assert!(c.apply_continuous(&["mock-1".into()], 50).is_err());
    assert_eq!(c.driver.writes, 0);
    assert!(c.changed.is_empty());
}
#[test]
fn api_success_is_not_readback_or_visible_effect() {
    let mut c = Controller::new(Mock {
        ignored_set: true,
        ..Default::default()
    })
    .unwrap();
    let out = c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(out[0].api_success);
    assert_eq!(out[0].readback_matches, Some(false));
    c.restore_all().unwrap();
}
#[test]
fn failed_api_still_restores_potentially_changed_display() {
    let mut c = Controller::new(Mock {
        fail_set: true,
        ..Default::default()
    })
    .unwrap();
    let out = c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(!out[0].api_success);
    assert!(c.changed.contains("mock-1"));
    c.restore_all().unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn all_monitors_apply_and_restore() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.apply_continuous(&["mock-1".into(), "mock-2".into()], 50)
        .unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.restored.len(), 2);
}
