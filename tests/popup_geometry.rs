//! Session geometry is separate from display controls; these tests are memory only.
use screen_bright_controller::ui::{popup_bounds, popup_on_screen};

#[test]
fn reopening_keeps_native_geometry_until_no_screen_shows_the_title_strip() {
    let areas = [(0, 0, 1920, 1040), (-1920, 0, 1920, 1040)];
    // Moved or resized anywhere a title strip stays visible, including across monitors.
    assert!(popup_on_screen(&areas, (-1600, 120, 720, 600)));
    assert!(popup_on_screen(&areas, (-300, 900, 720, 600)));
    assert!(popup_on_screen(&areas, (1900, 1020, 720, 600)));
    // Its monitor was removed, or the strip is entirely above or beside every work area.
    assert!(!popup_on_screen(&areas[..1], (-1600, 120, 720, 600)));
    assert!(!popup_on_screen(&areas, (100, -24, 720, 600)));
    assert!(!popup_on_screen(&areas, (1920, 100, 720, 600)));
    assert!(!popup_on_screen(&[], (0, 0, 720, 600)));
}

#[test]
fn popup_clamps_physical_work_area_at_mixed_dpi_and_negative_origin() {
    assert_eq!(
        popup_bounds((-1920, 0, 1920, 1040), (-10.0, 1030.0), 1.5),
        (-673, 502, 645, 510)
    );
    assert_eq!(
        popup_bounds((0, 40, 300, 400), (2.0, 45.0), 2.0),
        (0, 40, 300, 400)
    );
}
