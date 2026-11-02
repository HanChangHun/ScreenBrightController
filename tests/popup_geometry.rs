//! Session geometry is separate from display controls; these tests are memory only.
use screen_bright_controller::ui::{
    focus_loss_hides_popup, popup_bounds, popup_on_screen, popup_position, save_popup_geometry,
    saved_popup_geometry, size_request, tray_click_shows_popup,
};

#[test]
fn tray_anchor_keeps_the_real_window_above_the_taskbar() {
    // This PC: the primary work area ends at the taskbar (y 1032). Asking Tauri for 430x340
    // gave a 446x379 window, so placement uses the measured outer size.
    let (x, y) = popup_position((0, 0, 1920, 1032), (1700.0, 1056.0), (446, 379), 1.0);
    assert_eq!((x, y), (1242, 641));
    assert!(
        y + 379 <= 1032 - 12,
        "a DPI-scaled 12 px clear of the taskbar"
    );
    assert_eq!(
        popup_position((-1920, 0, 1920, 1040), (-10.0, 1030.0), (669, 560), 1.5),
        (-697, 452)
    );
    // A window larger than the work area keeps its top-left corner on screen.
    assert_eq!(
        popup_position((0, 40, 300, 400), (2.0, 45.0), (310, 410), 2.0),
        (0, 40)
    );
}

#[test]
fn saved_popup_geometry_round_trips_and_rejects_bad_or_hidden_rects() {
    let areas = [(0, 0, 1920, 1032), (1920, -406, 1080, 1872)];
    for rect in [(1076, 654, 446, 379), (2000, -300, 500, 400)] {
        assert_eq!(
            saved_popup_geometry(&save_popup_geometry(rect), &areas),
            Some(rect)
        );
    }
    // Off every screen (its monitor was removed): anchor to the tray instead.
    let gone = save_popup_geometry((4000, 100, 446, 379));
    assert_eq!(saved_popup_geometry(&gone, &areas), None);
    for bad in [
        "",
        "{",
        r#"{"x":1,"y":2,"width":0,"height":379}"#,
        r#"{"x":1,"y":2,"width":446,"height":99999}"#,
        r#"{"x":1,"y":2,"width":446,"height":379,"path":"C:/x"}"#,
    ] {
        assert_eq!(saved_popup_geometry(bad, &areas), None, "{bad}");
    }
}

#[test]
fn size_request_reaches_the_saved_outer_size_despite_frame_overhead() {
    // Tauri sizes the client area: asking for 446x379 produced a 462x418 window here.
    assert_eq!(size_request((446, 379), (446, 379), (462, 418)), (430, 340));
    assert_eq!(size_request((446, 379), (446, 379), (446, 379)), (446, 379));
    assert_eq!(size_request((10, 10), (446, 379), (900, 900)), (1, 1));
}

#[test]
fn only_focus_moving_to_another_app_hides_the_popup() {
    let own = 4242;
    // Dragging or resizing the popup briefly moves focus to its own native window.
    assert!(!focus_loss_hides_popup(true, Some(own), own));
    // Another app, the desktop or the taskbar took focus, or nothing is in front.
    assert!(focus_loss_hides_popup(true, Some(77), own));
    assert!(focus_loss_hides_popup(true, None, own));
    // Already hidden: nothing to do.
    assert!(!focus_loss_hides_popup(false, Some(77), own));
}

#[test]
fn tray_click_that_took_focus_from_the_popup_does_not_reopen_it() {
    // Visible: the click hides it (toggle).
    assert!(!tray_click_shows_popup(true, None));
    // Pressing the tray icon took focus, so focus loss already hid the popup.
    assert!(!tray_click_shows_popup(false, Some(0)));
    assert!(!tray_click_shows_popup(false, Some(500)));
    // Hidden earlier by focus loss, −, Escape or never shown: the click opens it.
    assert!(tray_click_shows_popup(false, Some(501)));
    assert!(tray_click_shows_popup(false, None));
}

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
