use gamma_dimmer::scale;
#[test]
fn dimming_conversion_accepts_every_integer() {
    for dim in 0..=90 {
        assert_eq!(
            gamma_dimmer::dimming_percent(dim).unwrap() as i64,
            100 - dim
        );
    }
    for dim in [-5, -1, 91, 95, 100, 256] {
        assert!(
            gamma_dimmer::dimming_percent(dim).is_err(),
            "accepted {dim}"
        );
    }
}
#[test]
fn unchanged_preview_does_not_arm_write_or_restore() {
    use gamma_dimmer::controller::{Controller, Mock};
    let mut c = Controller::new(Mock::default()).unwrap();
    c.preview(&["mock-1".into()], 100, 15, true).unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.writes, 0);
    assert!(c.driver.armed.is_empty());
    assert!(c.driver.restored.is_empty());
}
#[test]
fn authorized_ten_percent_scales_saved_channels() {
    let original = vec![vec![1000; 256], vec![2000; 256], vec![4000; 256]];
    let result = scale(&original, 10).unwrap();
    assert_eq!(result[0][100], 100);
    assert_eq!(result[1][100], 200);
    assert_eq!(result[2][100], 400);
    assert!(scale(&original, 9).is_err());
}
