#![cfg(windows)]
use screen_bright_controller::{session::Session, ui::UiSession};
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

#[test]
fn shared_controls_start_zero_validate_and_preview_effective_targets() {
    let mut ui = UiSession::new(Session::demo().unwrap()).unwrap();
    let status = ui.status().unwrap();
    assert_eq!(status["controls"]["master"]["dim"], 0);
    assert_eq!(status["consent"], false);
    assert!(ui.set_control("master", 91, true).is_err());
    assert!(ui.set_control("missing", 50, true).is_err());
    assert_eq!(ui.status().unwrap()["controls"]["master"]["dim"], 0);
    ui.set_control("master", 90, true).unwrap();
    ui.set_control("mock-2", 0, false).unwrap();
    assert!(ui.preview().is_err());
    ui.consent = true;
    ui.preview().unwrap();
    let status = ui.status().unwrap();
    assert_eq!(status["armed"], serde_json::json!(["mock-1"]));
    assert_eq!(status["consent"], false);
    assert_eq!(status["outcomes"][0]["readback_matches"], true);
    ui.restore().unwrap();
    ui.set_control("master", 0, false).unwrap();
    ui.set_control("mock-1", 40, true).unwrap();
    ui.set_control("mock-2", 90, true).unwrap();
    ui.consent = true;
    ui.preview().unwrap();
    assert_eq!(
        ui.status().unwrap()["outcomes"].as_array().unwrap().len(),
        2
    );
    ui.restore().unwrap();
}
