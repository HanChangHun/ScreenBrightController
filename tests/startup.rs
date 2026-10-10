use screen_bright_controller::startup::{
    get, launch_mode, registration_command, set, LaunchMode, Registry,
};
#[test]
fn set_failure_returns_actual_registration_and_error() {
    let mut m = Memory {
        fail_write: true,
        ..Default::default()
    };
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, Some(false));
    assert!(s.error.unwrap().contains("set failed"));
}
#[test]
fn write_without_effect_is_reported_from_readback() {
    let mut m = Memory {
        ignore_write: true,
        ..Default::default()
    };
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, Some(false));
    assert!(s.error.unwrap().contains("did not take effect"));
}
#[test]
fn get_failure_disables_unknown_registration_without_writing() {
    let m = Memory {
        fail_read: true,
        ..Default::default()
    };
    let s = get(&m, "expected");
    assert_eq!(s.enabled, None);
    assert!(s.error.unwrap().contains("get failed"));
    assert_eq!(m.writes, 0);
}
#[test]
fn startup_arguments_are_exact_and_never_route_to_watchdog() {
    assert_eq!(launch_mode(&[]), LaunchMode::Tray);
    assert_eq!(launch_mode(&["--autostart".into()]), LaunchMode::Tray);
    assert_eq!(launch_mode(&["--demo".into()]), LaunchMode::Demo);
    for arg in ["--watchdog", "--diagnose", "--self-test"] {
        assert_eq!(launch_mode(&[arg.into()]), LaunchMode::Cli);
        assert_eq!(
            launch_mode(&["--autostart".into(), arg.into()]),
            LaunchMode::Cli
        );
    }
    assert_eq!(
        registration_command("C:\\Apps With Spaces\\ScreenBrightController.exe").unwrap(),
        "\"C:\\Apps With Spaces\\ScreenBrightController.exe\" --autostart"
    );
    assert!(registration_command("bad\"path.exe").is_err());
    let mut session = screen_bright_controller::ui::UiSession::new(
        screen_bright_controller::session::Session::demo().unwrap(),
    )
    .unwrap();
    assert_eq!(session.status().unwrap()["armed"], serde_json::json!([]));
    session.heartbeat().unwrap();
    assert_eq!(session.status().unwrap()["controls"]["master"]["dim"], 0);
}
#[derive(Default)]
struct Memory {
    value: Option<String>,
    writes: usize,
    fail_read: bool,
    fail_write: bool,
    ignore_write: bool,
}
impl Registry for Memory {
    fn read(&self) -> Result<Option<String>, String> {
        if self.fail_read {
            return Err("get failed".into());
        }
        Ok(self.value.clone())
    }
    fn write(&mut self, value: Option<&str>) -> Result<(), String> {
        if self.fail_write {
            return Err("set failed".into());
        }
        self.writes += 1;
        if !self.ignore_write {
            self.value = value.map(str::to_owned);
        }
        Ok(())
    }
}
#[test]
fn explicit_toggle_is_verified_and_disable_is_idempotent() {
    let mut m = Memory::default();
    let expected = "\"C:\\Apps With Spaces\\ScreenBrightController.exe\" --autostart";
    assert_eq!(get(&m, expected).enabled, Some(false));
    assert_eq!(m.writes, 0);
    assert_eq!(set(&mut m, expected, true).enabled, Some(true));
    assert_eq!(m.value.as_deref(), Some(expected));
    assert_eq!(set(&mut m, expected, false).enabled, Some(false));
    assert!(m.value.is_none());
    assert_eq!(set(&mut m, expected, false).enabled, Some(false));
}
#[test]
fn current_registration_is_read_not_a_cached_default() {
    let m = Memory {
        value: Some("\"C:\\Apps With Spaces\\ScreenBrightController.exe\" --autostart".into()),
        ..Default::default()
    };
    let state = get(
        &m,
        "\"C:\\Apps With Spaces\\ScreenBrightController.exe\" --autostart",
    );
    assert_eq!(state.enabled, Some(true));
    assert!(state.error.is_none());
    assert_eq!(m.writes, 0);
}
