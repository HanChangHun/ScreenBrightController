//! Plain-text diagnostic log for the tray app (`app.log`) and its real watchdog
//! (`watchdog.log`). Writing never affects gamma decisions: every failure is ignored.
//! Only `init` enables a file, so tests, demo and mock runs write nothing.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};
/// Tauri's app data directory name (`identifier` in tauri.conf.json).
pub const IDENTIFIER: &str = "io.github.screenbrightcontroller";
/// The previous file is kept as `<name>.1` once a file reaches this size.
pub const MAX_BYTES: u64 = 1024 * 1024;
static FILE: OnceLock<PathBuf> = OnceLock::new();
/// `%LOCALAPPDATA%\io.github.screenbrightcontroller\logs`
pub fn dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(IDENTIFIER).join("logs"))
}
/// Log to `name` in `dir()` from now on, and record panics there too.
pub fn init(name: &str) {
    let Some(path) = dir().map(|d| d.join(name)) else {
        return;
    };
    if FILE.set(path).is_ok() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            write(format!("panic: {info}"));
            previous(info);
        }));
    }
}
pub fn write(message: impl AsRef<str>) {
    if let Some(path) = FILE.get() {
        append(path, &line(message.as_ref()), MAX_BYTES);
    }
}
/// One local-time stamped line with the process ID; line breaks cannot forge entries.
pub fn line(message: &str) -> String {
    let mut t = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} [{}] {}\n",
        t.wYear,
        t.wMonth,
        t.wDay,
        t.wHour,
        t.wMinute,
        t.wSecond,
        t.wMilliseconds,
        std::process::id(),
        message.replace(['\r', '\n'], " ")
    )
}
/// Append `line`, first moving a file of at least `max_bytes` to `<name>.1`.
pub fn append(path: &Path, line: &str, max_bytes: u64) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::metadata(path).is_ok_and(|m| m.len() >= max_bytes) {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".1");
        let _ = fs::rename(path, backup);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
