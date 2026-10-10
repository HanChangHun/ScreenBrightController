use screen_bright_controller::{
    controller::{Controller, Mock},
    guard::Guard,
};
#[test]
fn continuous_lease_renews_without_writes_and_expires() {
    let mut g = Guard::new(Mock::default()).unwrap();
    g.arm_continuous("mock-1", 10, 0).unwrap();
    g.renew("mock-1", 10, 8000).unwrap();
    assert_eq!(g.driver.writes, 0);
    g.tick(15000, true);
    assert!(g.armed.contains_key("mock-1"));
    g.tick(18000, true);
    assert!(g.armed.is_empty());
    assert_eq!(g.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn continuous_update_preserves_original_zero_restores_and_idle_zero_no_write() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.apply_continuous(&["mock-1".into()], 100).unwrap();
    assert_eq!(c.driver.writes, 0);
    c.apply_continuous(&["mock-1".into()], 70).unwrap();
    c.apply_continuous(&["mock-1".into()], 40).unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 16000);
    c.apply_continuous(&["mock-1".into()], 100).unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
    assert!(c.changed.is_empty());
}
#[test]
fn expired_failed_restore_cannot_be_rescued_by_heartbeat() {
    let mut g = Guard::new(Mock {
        fail_restore: true,
        ..Default::default()
    })
    .unwrap();
    g.arm_continuous("mock-1", 10, 0).unwrap();
    g.tick(10000, true);
    assert!(g.renew("mock-1", 10, 10001).is_err());
    g.driver.fail_restore = false;
    g.tick(11000, true);
    assert!(g.armed.is_empty());
}
#[test]
fn heartbeat_detects_external_change_without_fighting() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.apply_continuous(&["mock-1".into()], 70).unwrap();
    let writes = c.driver.writes;
    c.heartbeat().unwrap();
    assert_eq!(writes, c.driver.writes);
    c.driver
        .current
        .insert("mock-1".into(), vec![vec![333; 256]; 3]);
    assert!(c.heartbeat().is_err());
    assert!(c.changed.is_empty());
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn watchdog_restored_lease_is_reported_and_forgotten() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.apply_continuous(&["mock-1".into(), "mock-2".into()], 70)
        .unwrap();
    let error = c.release_unarmed(&["mock-2".into()]).unwrap_err();
    assert!(
        error.contains("mock-1") && !error.contains("mock-2"),
        "{error}"
    );
    assert!(!c.changed.contains("mock-1") && !c.expected.contains_key("mock-1"));
    assert!(c.changed.contains("mock-2") && c.expected.contains_key("mock-2"));
    c.release_unarmed(&["mock-2".into()]).unwrap();
}
