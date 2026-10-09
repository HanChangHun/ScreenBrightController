//! Session geometry is separate from display controls; these tests are memory only.
use screen_bright_controller::ui::{popup_reopen_bounds, PopupArea};

#[test]
fn reopening_keeps_native_user_geometry_instead_of_tray_defaults() {
    let areas = [PopupArea {
        work: (-1920, 0, 1920, 1040),
        scale: 1.0,
    }];
    assert_eq!(
        popup_reopen_bounds(&areas, (-1600, 120, 720, 600)),
        Some((0, (-1600, 120, 720, 600)))
    );
    // Repeating hide/reopen is idempotent; there is no serialized restart preference.
    let (_, retained) = popup_reopen_bounds(&areas, (-1600, 120, 720, 600)).unwrap();
    assert_eq!(popup_reopen_bounds(&areas, retained), Some((0, retained)));
}

#[test]
fn screen_changes_clamp_to_the_best_available_work_area_without_resetting_size() {
    let areas = [
        PopupArea {
            work: (0, 40, 1920, 1000),
            scale: 1.0,
        },
        PopupArea {
            work: (-1920, 0, 1920, 1040),
            scale: 1.5,
        },
    ];
    assert_eq!(
        popup_reopen_bounds(&areas, (-1600, 850, 720, 600)),
        Some((1, (-1600, 440, 720, 600)))
    );
    // Removed monitor: recover nearest remaining work area, retaining resized dimensions.
    assert_eq!(
        popup_reopen_bounds(&areas[..1], (-1600, 120, 720, 600)),
        Some((0, (0, 120, 720, 600)))
    );
    // DPI-aware minimums, but work area takes priority over minimum on a tiny screen.
    assert_eq!(
        popup_reopen_bounds(&areas[1..], (-1600, 120, 430, 260)),
        Some((0, (-1600, 120, 645, 390)))
    );
    let tiny = [PopupArea {
        work: (10, -300, 300, 200),
        scale: 2.0,
    }];
    assert_eq!(
        popup_reopen_bounds(&tiny, (-100, 500, 900, 700)),
        Some((0, (10, -300, 300, 200)))
    );
    assert_eq!(popup_reopen_bounds(&[], (0, 0, 720, 600)), None);
}
