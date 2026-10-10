use screen_bright_controller::scale;
#[test]
fn dimming_conversion_accepts_every_integer() {
    for dim in 0..=90 {
        assert_eq!(
            screen_bright_controller::dimming_percent(dim).unwrap() as i64,
            100 - dim
        );
    }
    for dim in [-5, -1, 91, 95, 100, 256] {
        assert!(
            screen_bright_controller::dimming_percent(dim).is_err(),
            "accepted {dim}"
        );
    }
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
