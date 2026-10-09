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
pub fn set(registry: &mut impl Registry, expected: &str, enabled: bool) -> Status {
    let previous = match registry.read() {
        Ok(value) => value,
        Err(error) => {
            return Status {
                enabled: None,
                error: Some(format!("Startup state unavailable: {error}")),
            }
        }
    };
    let target = enabled.then_some(expected);
    let result = registry.write(target).and_then(|()| {
        let actual = registry.read()?;
        if actual.as_deref() == target {
            Ok(())
        } else {
            Err("Startup readback mismatch".into())
        }
    });
    match result {
        Ok(()) => Status {
            enabled: Some(enabled),
            error: None,
        },
        Err(mut error) => {
            if let Err(e) = registry.write(previous.as_deref()) {
                error.push_str(&format!("; rollback failed: {e}"));
            }
            // The same final readback supplies both rollback verification and UI state.
            let enabled = match registry.read() {
                Ok(actual) => {
                    if actual == previous {
                        error.push_str("; previous registration restored");
                    } else {
                        error.push_str("; rollback readback mismatch");
                    }
                    Some(actual.as_deref() == Some(expected))
                }
                Err(e) => {
                    error.push_str(&format!("; rollback readback unavailable: {e}"));
                    None
                }
            };
            Status {
                enabled,
                error: Some(error),
            }
        }
    }
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
