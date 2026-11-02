#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod tray_wheel;
mod update;
use screen_bright_controller::startup::{self, launch_mode, LaunchMode};
use screen_bright_controller::{
    log,
    session::Session,
    ui::{
        focus_loss_hides_popup, popup_bounds, popup_on_screen, popup_position, save_popup_geometry,
        saved_popup_geometry, size_request, tray_click_shows_popup, PopupRect, Status, UiSession,
    },
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, OnceLock,
};
use std::time::Instant;
static STARTUP_LOCK: Mutex<()> = Mutex::new(());
#[tauri::command]
async fn get_autostart() -> Result<startup::Status, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let _guard = STARTUP_LOCK.lock().map_err(|_| "Startup lock failed")?;
        Ok(startup::get(
            &startup::windows::WindowsRegistry,
            &startup::windows::current_command()?,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_autostart(enabled: bool) -> Result<startup::Status, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STARTUP_LOCK.lock().map_err(|_| "Startup lock failed")?;
        Ok(startup::set(
            &mut startup::windows::WindowsRegistry,
            &startup::windows::current_command()?,
            enabled,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
type State = Mutex<Result<UiSession, String>>;
const TRAY_ID: &str = "screen-bright-controller";
/// Main thread only: whether the tray menu currently offers Quit anyway.
static QUIT_ANYWAY: AtomicBool = AtomicBool::new(false);
/// Main thread only: the popup has been anchored to the tray once.
static POPUP_PLACED: AtomicBool = AtomicBool::new(false);
/// Main thread only: when focus loss last hid the popup, in ms since `CLOCK` (0 = never).
static FOCUS_HIDDEN_MS: AtomicU64 = AtomicU64::new(0);
static CLOCK: OnceLock<Instant> = OnceLock::new();
fn clock_ms() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}
/// Process ID of the foreground window, if any. Read-only.
fn foreground_pid() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    (pid != 0).then_some(pid)
}
struct QuitItem(MenuItem<tauri::Wry>);
fn tray_tooltip(attention: bool) -> &'static str {
    if attention {
        "Screen Bright Controller · attention needed"
    } else {
        "Screen Bright Controller"
    }
}
fn quit_label(detached: bool) -> &'static str {
    if detached {
        "Quit anyway (restores unplugged displays when they return)"
    } else {
        "Restore and quit"
    }
}
fn with_session<T>(
    state: &State,
    f: impl FnOnce(&mut UiSession) -> Result<T, String>,
) -> Result<T, String> {
    f(state
        .lock()
        .map_err(|_| "state lock failed")?
        .as_mut()
        .map_err(|e| e.clone())?)
}
#[tauri::command]
fn status(state: tauri::State<'_, State>) -> Result<Status, String> {
    with_session(&state, UiSession::status)
}
/// Publish the current recovery state to the tray. Callers must have released the state
/// lock: this runs inline on the main thread, and is queued there from workers.
fn refresh_attention(app: &tauri::AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let (attention, detached) = handle.state::<State>().lock().map_or((true, false), |s| {
            s.as_ref()
                .map_or((true, false), |s| (s.needs_attention(), s.detached()))
        });
        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(tray_tooltip(attention)));
        }
        if QUIT_ANYWAY.swap(detached, Ordering::SeqCst) != detached {
            if let Some(quit) = handle.try_state::<QuitItem>() {
                let _ = quit.0.set_text(quit_label(detached));
            }
        }
    });
}
fn restore_app(state: &State, app: &tauri::AppHandle, source: &str) -> Result<(), String> {
    log::write(format!("Restore requested from {source}"));
    // Without a session nothing was dimmed, so Quit must not be blocked.
    let result = match state.lock().map_err(|_| "state lock failed")?.as_mut() {
        Ok(s) => s.restore(),
        Err(_) => Ok(()),
    };
    refresh_attention(app);
    result
}
#[tauri::command]
fn live_control(
    id: String,
    dim: i64,
    enabled: bool,
    generation: u64,
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<Status, String> {
    let result = with_session(&state, |s| {
        s.live_control(&id, dim, enabled, generation)?;
        s.status()
    });
    refresh_attention(&app);
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn restore(state: tauri::State<'_, State>, app: tauri::AppHandle) -> Result<Status, String> {
    let result = restore_app(&state, &app, "the popup");
    let _ = app.emit("state-changed", ());
    result?;
    with_session(&state, UiSession::status)
}
#[tauri::command]
fn reset_baseline(state: tauri::State<'_, State>, app: tauri::AppHandle) -> Result<Status, String> {
    let result = with_session(&state, |s| {
        s.reset_baseline()?;
        s.status()
    });
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn hide_popup(app: tauri::AppHandle) {
    remember_popup(&app);
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.hide();
    }
}
/// Where the popup's outer geometry is kept between runs.
fn geometry_file(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    Some(
        app.path()
            .app_local_data_dir()
            .ok()?
            .join("popup-geometry.json"),
    )
}
/// Main thread only: record where the visible popup is before it hides or the app exits.
fn remember_popup(app: &tauri::AppHandle) {
    let Some(w) = app.get_webview_window("popup") else {
        return;
    };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let (Ok(position), Ok(size), Some(file)) =
        (w.outer_position(), w.outer_size(), geometry_file(app))
    else {
        return;
    };
    let text = save_popup_geometry((position.x, position.y, size.width, size.height));
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&file, text) {
        log::write(format!("popup position not saved: {e}"));
    }
}
fn report(app: &tauri::AppHandle, error: String) {
    log::write(format!("error: {error}"));
    // Errors may be obsolete or unrelated to display recovery. Re-read current state
    // and publish under its lock rather than letting a delayed report undo Restore.
    refresh_attention(app);
    let _ = app.emit("display-error", error);
}
fn work_area(monitor: &tauri::Monitor) -> PopupRect {
    let work = monitor.work_area();
    (
        work.position.x,
        work.position.y,
        work.size.width,
        work.size.height,
    )
}
/// Give the popup this outer rect. set_size sizes the client area, so one measurement
/// corrects the request until the window itself has the saved size.
fn set_outer(w: &tauri::WebviewWindow, (x, y, width, height): PopupRect) -> Result<(), String> {
    // Move first: crossing to another DPI rescales the window, then the size is exact.
    w.set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    w.set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let actual = w.outer_size().map_err(|e| e.to_string())?;
    let request = size_request(
        (width, height),
        (width, height),
        (actual.width, actual.height),
    );
    if request != (width, height) {
        w.set_size(tauri::PhysicalSize::new(request.0, request.1))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
/// Size the popup for the tray's monitor, then place the measured window beside the click
/// and clear of the taskbar.
fn anchor_popup(
    app: &tauri::AppHandle,
    w: &tauri::WebviewWindow,
    click: tauri::PhysicalPosition<f64>,
) -> Result<(), String> {
    let monitor = app
        .monitor_from_point(click.x, click.y)
        .map_err(|e| e.to_string())?
        .ok_or("tray monitor unavailable")?;
    let (work, scale) = (work_area(&monitor), monitor.scale_factor());
    let (x, y, width, height) = popup_bounds(work, (click.x, click.y), scale);
    // Move first: crossing to another DPI rescales the window, then the size is exact.
    w.set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    w.set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let outer = w.outer_size().map_err(|e| e.to_string())?;
    let (x, y) = popup_position(work, (click.x, click.y), (outer.width, outer.height), scale);
    w.set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}
/// The first open restores the geometry saved by the last run, or anchors to the tray. Later
/// opens keep the native geometry the user chose and re-anchor only when no work area shows
/// the popup's top strip (e.g. its monitor left).
fn show_popup(app: &tauri::AppHandle, click: tauri::PhysicalPosition<f64>) -> Result<(), String> {
    let w = app.get_webview_window("popup").ok_or("popup unavailable")?;
    let areas: Vec<_> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .map(work_area)
        .collect();
    let position = w.outer_position().map_err(|e| e.to_string())?;
    let size = w.outer_size().map_err(|e| e.to_string())?;
    let current = (position.x, position.y, size.width, size.height);
    if !POPUP_PLACED.load(Ordering::SeqCst) {
        let saved = geometry_file(app)
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|text| saved_popup_geometry(&text, &areas));
        match saved {
            Some(rect) => set_outer(&w, rect)?,
            None => anchor_popup(app, &w, click)?,
        }
        POPUP_PLACED.store(true, Ordering::SeqCst);
    } else if !popup_on_screen(&areas, current) {
        anchor_popup(app, &w, click)?;
    }
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit("state-changed", ());
    Ok(())
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--autostart-check"] {
        let result = startup::windows::current_command()
            .map(|expected| startup::get(&startup::windows::WindowsRegistry, &expected));
        match result {
            Ok(state) => println!(
                "{}",
                serde_json::json!({"startup":state,"native_display_writes":0,"registration_writes":0})
            ),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1)
            }
        }
        return;
    }
    let mode = launch_mode(&args);
    if mode == LaunchMode::Cli {
        if let Err(e) = screen_bright_controller::cli::run(&args) {
            eprintln!("{e}");
            std::process::exit(1)
        }
        return;
    }
    let demo = mode == LaunchMode::Demo;
    if !demo {
        log::init("app.log");
        log::write(format!(
            "app {} started{}",
            env!("CARGO_PKG_VERSION"),
            if args.is_empty() {
                ""
            } else {
                " at Windows login"
            }
        ));
    }
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .manage(Mutex::new(Err::<UiSession, String>(
            "starting read-only snapshot".into(),
        )))
        .manage(update::Updates::default())
        .invoke_handler(tauri::generate_handler![
            update::check_update,
            update::install_update,
            get_autostart,
            set_autostart,
            status,
            live_control,
            restore,
            reset_baseline,
            hide_popup
        ])
        .setup(move |app| {
            let session = if demo {
                Session::demo()
            } else {
                Session::real()
            };
            let session = session.and_then(UiSession::new);
            if let Err(e) = &session {
                log::write(format!("display session unavailable: {e}"));
            }
            *app.state::<State>().lock().unwrap() = session;
            let worker = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_error = String::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let result = with_session(&worker.state::<State>(), UiSession::heartbeat);
                    refresh_attention(&worker);
                    match result {
                        Ok(()) => last_error.clear(),
                        Err(e) if e != last_error => {
                            last_error = e.clone();
                            report(&worker, e);
                        }
                        Err(_) => {}
                    }
                    let _ = worker.emit("state-changed", ());
                }
            });

            let restore_item = MenuItem::with_id(
                app,
                "restore",
                "Restore saved original gamma",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", quit_label(false), true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&restore_item, &quit_item])?;
            app.manage(QuitItem(quit_item));
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tauri::include_image!("icons/tray.png"))
                .tooltip(tray_tooltip(false))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        position,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let visible = app
                            .get_webview_window("popup")
                            .is_some_and(|w| w.is_visible().unwrap_or(false));
                        let hidden = FOCUS_HIDDEN_MS.load(Ordering::SeqCst);
                        let since = (hidden > 0).then(|| clock_ms().saturating_sub(hidden));
                        if visible {
                            remember_popup(app);
                            if let Some(w) = app.get_webview_window("popup") {
                                let _ = w.hide();
                            }
                        } else if tray_click_shows_popup(false, since) {
                            if let Err(e) = show_popup(app, position) {
                                report(app, e);
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "restore" => {
                        let result = restore_app(&app.state::<State>(), app, "the tray menu");
                        if let Err(e) = result {
                            report(app, e);
                        }
                        let _ = app.emit("state-changed", ());
                    }
                    // ExitRequested restores first; see there for when a failure cancels the exit.
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if !demo {
                if let Err(error) = tray_wheel::start(app.handle()) {
                    report(app.handle(), error);
                }
            }
            refresh_attention(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "popup" {
                return;
            }
            // Close and focus loss hide the popup like − and Escape; dimming continues.
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    remember_popup(window.app_handle());
                    let _ = window.hide();
                }
                tauri::WindowEvent::Focused(false)
                    if focus_loss_hides_popup(
                        window.is_visible().unwrap_or(false),
                        foreground_pid(),
                        std::process::id(),
                    ) =>
                {
                    FOCUS_HIDDEN_MS.store(clock_ms(), Ordering::SeqCst);
                    remember_popup(window.app_handle());
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri application failed to initialize");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            remember_popup(app);
            // Read what the menu offered before this attempt republishes it.
            let offered = QUIT_ANYWAY.load(Ordering::SeqCst);
            let state = app.state::<State>();
            if let Err(e) = restore_app(&state, app, "quit") {
                // Quit anyway: only unplugged displays remain, and the running watchdog keeps
                // their originals and restores them when they return. Anything else cancels.
                let detached = state
                    .lock()
                    .is_ok_and(|s| s.as_ref().is_ok_and(UiSession::detached));
                if !(offered && detached) {
                    api.prevent_exit();
                    log::write("quit cancelled: Restore failed");
                    report(app, e)
                } else {
                    log::write(
                        "quit anyway: the watchdog restores unplugged displays when they return",
                    );
                }
            } else {
                log::write("app exiting");
            }
        }
    });
}
