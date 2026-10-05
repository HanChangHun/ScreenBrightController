use gamma_dimmer::{
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
fn preview_cannot_be_renewed_or_promoted() {
    let mut g = Guard::new(Mock::default()).unwrap();
    g.arm("mock-1", 15, 0).unwrap();
    assert!(g.renew("mock-1", 10, 1).is_err());
    assert!(g.arm_continuous("mock-1", 10, 1).is_err());
    assert!(g.arm_continuous("mock-2", 11, 1).is_err());
}
#[test]
fn continuous_update_preserves_original_zero_restores_and_idle_zero_no_write() {
    let mut c = Controller::new(Mock::default()).unwrap();
    assert!(c.apply_continuous(&["mock-1".into()], 70, false).is_err());
    c.apply_continuous(&["mock-1".into()], 100, true).unwrap();
    assert_eq!(c.driver.writes, 0);
    c.apply_continuous(&["mock-1".into()], 70, true).unwrap();
    c.apply_continuous(&["mock-1".into()], 40, true).unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 16000);
    c.apply_continuous(&["mock-1".into()], 100, true).unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
    assert!(c.changed.is_empty());
}
#[test]
fn ui_continuous_close_hides_exclusion_restores_and_restore_resets_values() {
    use gamma_dimmer::{session::Session, ui::UiSession};
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    ui.set_control("master", 40, true).unwrap();
    assert!(ui.apply("continuous").is_err());
    ui.consent = true;
    assert!(ui.apply("invalid").is_err());
    ui.apply("continuous").unwrap();
    ui.main_close().unwrap();
    assert_eq!(ui.status().unwrap()["armed"].as_array().unwrap().len(), 2);
    ui.set_control("mock-1", 40, false).unwrap();
    assert_eq!(ui.status().unwrap()["armed"], serde_json::json!(["mock-2"]));
    ui.restore().unwrap();
    assert_eq!(ui.status().unwrap()["controls"]["master"]["dim"], 0);
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
    c.apply_continuous(&["mock-1".into()], 70, true).unwrap();
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
