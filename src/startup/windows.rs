//! Standard per-user HKCU Run registration. No shell, elevation, tasks, or gamma calls.
use super::{registration_command, Registry};
use std::io::ErrorKind;
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE},
    RegKey,
};
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const ENTRY: &str = "ScreenBrightController";
pub struct WindowsRegistry;
impl Registry for WindowsRegistry {
    fn read(&self) -> Result<Option<String>, String> {
        let result = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ)
            .and_then(|key| key.get_value::<String, _>(ENTRY));
        match result {
            Ok(value) => Ok(Some(value)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn write(&mut self, value: Option<&str>) -> Result<(), String> {
        let user = RegKey::predef(HKEY_CURRENT_USER);
        match value {
            Some(value) => {
                let (key, _) = user
                    .create_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
                    .map_err(|e| e.to_string())?;
                key.set_value(ENTRY, &value).map_err(|e| e.to_string())
            }
            None => match user
                .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
                .and_then(|key| key.delete_value(ENTRY))
            {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            },
        }
    }
}
pub fn current_command() -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    registration_command(
        executable
            .to_str()
            .ok_or("Executable path is not valid Unicode")?,
    )
}
