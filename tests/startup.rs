use gamma_dimmer::startup::{get, launch_mode, registration_command, set, LaunchMode, Registry};
use std::cell::Cell;
struct Fault {
    inner: Memory,
    reads: Cell<usize>,
    fail_read: Option<usize>,
    fail_write: bool,
    mismatch: bool,
}
impl Registry for Fault {
    fn read(&self) -> Result<Option<String>, String> {
        let n = self.reads.get() + 1;
        self.reads.set(n);
        if self.fail_read == Some(n) {
            Err("get failed".into())
        } else {
            self.inner.read()
        }
    }
    fn write(&mut self, v: Option<&str>) -> Result<(), String> {
        if self.fail_write {
            return Err("set failed".into());
        }
        if self.mismatch && self.inner.writes == 0 {
            self.inner.writes += 1;
            Ok(())
        } else {
            self.inner.write(v)
        }
    }
}
fn fault() -> Fault {
    Fault {
        inner: Memory::default(),
        reads: Cell::new(0),
        fail_read: None,
        fail_write: false,
        mismatch: false,
    }
}
#[test]
fn set_failure_returns_actual_registration_and_error() {
    let mut m = fault();
    m.fail_write = true;
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, Some(false));
    assert!(s.error.unwrap().contains("set failed"));
}
#[test]
fn mismatch_rolls_back_previous_raw_entry_and_reports_failure() {
    let mut m = fault();
    m.inner.value = Some("old location".into());
    m.mismatch = true;
    let s = set(&mut m, "expected", true);
    assert_eq!(m.inner.writes, 2);
    assert_eq!(m.inner.value.as_deref(), Some("old location"));
    assert_eq!(s.enabled, Some(false));
    assert!(s.error.unwrap().contains("readback mismatch"));
}
#[test]
fn rollback_unknown_readback_never_returns_stale_checkbox_state() {
    let mut m = fault();
    m.mismatch = true;
    m.fail_read = Some(3);
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, None);
    assert!(s.error.unwrap().contains("rollback readback unavailable"));
    assert_eq!(m.reads.get(), 3);
}
#[test]
fn get_failure_disables_unknown_registration_without_writing() {
    let mut m = fault();
    m.fail_read = Some(1);
    let s = get(&m, "expected");
    assert_eq!(s.enabled, None);
    assert!(s.error.unwrap().contains("get failed"));
    assert_eq!(m.inner.writes, 0);
}
#[test]
fn failed_initial_read_never_writes() {
    let mut m = fault();
    m.fail_read = Some(1);
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, None);
    assert_eq!(m.inner.writes, 0);
}
#[test]
fn failed_verification_rolls_back() {
    let mut m = fault();
    m.fail_read = Some(2);
    let s = set(&mut m, "expected", true);
    assert_eq!(s.enabled, Some(false));
    assert_eq!(m.inner.value, None);
    assert!(s.error.unwrap().contains("get failed"));
}
#[test]
fn startup_arguments_are_exact_and_never_route_to_watchdog() {
    assert_eq!(launch_mode(&[]), LaunchMode::Main);
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
    let mut session =
        gamma_dimmer::ui::UiSession::new(gamma_dimmer::session::Session::demo().unwrap()).unwrap();
    assert_eq!(session.status().unwrap()["armed"], serde_json::json!([]));
    session.heartbeat().unwrap();
    assert_eq!(session.status().unwrap()["controls"]["master"]["dim"], 0);
}
#[derive(Default)]
struct Memory {
    value: Option<String>,
    writes: usize,
}
impl Registry for Memory {
    fn read(&self) -> Result<Option<String>, String> {
        Ok(self.value.clone())
    }
    fn write(&mut self, value: Option<&str>) -> Result<(), String> {
        self.writes += 1;
        self.value = value.map(str::to_owned);
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
        writes: 0,
    };
    let state = get(
        &m,
        "\"C:\\Apps With Spaces\\ScreenBrightController.exe\" --autostart",
    );
    assert_eq!(state.enabled, Some(true));
    assert!(state.error.is_none());
    assert_eq!(m.writes, 0);
}
