//! Only check_update is automatic/read-only. install_update is the explicit button
//! path; no caller-provided URLs, commands or paths are accepted.
use crate::{restore_app, State};
use screen_bright_controller::{
    session::Session,
    update::{expected_checksum, verify_bytes, Release, LATEST, MAX_DOWNLOAD},
};
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    process::Command,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
#[derive(Default)]
pub struct Updates(pub Mutex<Info>);
#[derive(Default)]
pub struct Info {
    release: Option<Release>,
    busy: bool,
}
#[derive(Serialize)]
pub struct Check {
    current: &'static str,
    release: Option<Release>,
}
fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .https_only(true)
        .user_agent("ScreenBrightController-update")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5
                || attempt.url().scheme() != "https"
                || !matches!(
                    attempt.url().host_str(),
                    Some(
                        "github.com"
                            | "api.github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
            {
                attempt.error("Update redirect rejected")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}
fn fetch(client: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("Update response too large".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Update response too large".into());
    }
    Ok(bytes)
}
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<Check, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = fetch(&client()?, LATEST, 1024 * 1024)?;
        let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        let release = Release::parse(text, env!("CARGO_PKG_VERSION"))?;
        app.state::<Updates>()
            .0
            .lock()
            .map_err(|_| "Update state lock failed")?
            .release = release.clone();
        Ok(Check {
            current: env!("CARGO_PKG_VERSION"),
            release,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
fn stage(app: &tauri::AppHandle, release: &Release) -> Result<(), String> {
    let http = client()?;
    let bytes = fetch(&http, &release.checksum_url, 8192)?;
    let sums = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let hash = expected_checksum(sums, &release.name)?;
    let bytes = fetch(&http, &release.installer_url, MAX_DOWNLOAD)?;
    verify_bytes(&bytes, release.size, &hash)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("updates");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let folder = cache.join(format!("{}-{nonce}", std::process::id()));
    fs::create_dir(&folder).map_err(|e| e.to_string())?;
    let installer = folder.join(&release.name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&installer)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let script = folder.join("update-install.ps1");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&script)
        .map_err(|e| e.to_string())?;
    file.write_all(include_bytes!("update-install.ps1"))
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let watchdog = {
        let state = app.state::<State>();
        let guard = state.lock().map_err(|_| "State lock failed")?;
        match guard.as_ref() {
            Ok(s) => match &s.session {
                Session::Real(c) => c.driver.link.child.id(),
                Session::Demo(_) => return Err("Updates cannot install in demo mode".into()),
            },
            Err(_) => 0,
        }
    };
    // A failed Restore keeps the app and watchdog running and never launches setup.
    restore_app(&app.state::<State>(), app)?;
    let powershell = std::env::var_os("SystemRoot").ok_or("SystemRoot unavailable")?;
    let powershell =
        std::path::PathBuf::from(powershell).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .arg("-Installer")
        .arg(installer)
        .arg("-ExpectedHash")
        .arg(hash)
        .arg("-App")
        .arg(exe)
        .arg("-ParentPid")
        .arg(std::process::id().to_string())
        .arg("-WatchdogPid")
        .arg(watchdog.to_string())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let release = {
            let state = worker.state::<Updates>();
            let mut info = state.0.lock().map_err(|_| "Update state lock failed")?;
            if info.busy {
                return Err("An update is already being prepared".into());
            }
            let release = info.release.clone().ok_or("Check for an update first")?;
            info.busy = true;
            release
        };
        let result = stage(&worker, &release);
        if result.is_err() {
            if let Ok(mut info) = worker.state::<Updates>().0.lock() {
                info.busy = false;
            }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())??;
    // Normal ExitRequested performs Restore again and may cancel exit. The helper
    // waits for both process handles and refuses to install while either remains.
    app.exit(0);
    Ok(())
}
