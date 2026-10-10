//! Opt-in login registration, isolated from all gamma/session operations.
use serde::Serialize;
#[cfg(windows)]
pub mod windows;
#[derive(Debug, PartialEq, Eq)]
pub enum LaunchMode {
    Demo,
    Tray,
    Cli,
}
pub fn launch_mode(args: &[String]) -> LaunchMode {
    match args {
        [] => LaunchMode::Tray,
        [arg] if arg == "--autostart" => LaunchMode::Tray,
        [arg] if arg == "--demo" => LaunchMode::Demo,
        _ => LaunchMode::Cli,
    }
}
pub fn registration_command(executable: &str) -> Result<String, String> {
    if executable.is_empty() || executable.contains(['"', '\n', '\r']) {
        return Err("Invalid executable path".into());
    }
    Ok(format!("\"{executable}\" --autostart"))
}
pub trait Registry {
    fn read(&self) -> Result<Option<String>, String>;
    fn write(&mut self, value: Option<&str>) -> Result<(), String>;
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub enabled: Option<bool>,
    pub error: Option<String>,
}
/// Write, then report the registration actually read back.
pub fn set(registry: &mut impl Registry, expected: &str, enabled: bool) -> Status {
    let written = registry.write(enabled.then_some(expected));
    let mut status = get(registry, expected);
    if let Err(e) = written {
        status.error = Some(format!("Startup change failed: {e}"));
    } else if status.enabled.is_some() && status.enabled != Some(enabled) {
        status.error = Some("Startup change did not take effect".into());
    }
    status
}
pub fn get(registry: &impl Registry, expected: &str) -> Status {
    match registry.read() {
        Ok(None) => Status {
            enabled: Some(false),
            error: None,
        },
        Ok(Some(value)) if value == expected => Status {
            enabled: Some(true),
            error: None,
        },
        Ok(Some(_)) => Status {
            enabled: Some(false),
            error: Some(
                "Startup entry points to a different executable. Enable to update its location."
                    .into(),
            ),
        },
        Err(error) => Status {
            enabled: None,
            error: Some(format!("Startup state unavailable: {error}")),
        },
    }
}
