#![cfg(windows)]
#[test]
fn popup_clamps_physical_work_area_at_mixed_dpi_and_negative_origin() {
    use screen_bright_controller::ui::popup_bounds;
    assert_eq!(
        popup_bounds((-1920, 0, 1920, 1040), (-10.0, 1030.0), 1.5),
        (-673, 502, 645, 510)
    );
    assert_eq!(
        popup_bounds((0, 40, 300, 400), (2.0, 45.0), 2.0),
        (0, 40, 300, 400)
    );
}
