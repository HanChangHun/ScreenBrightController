#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use screen_bright_controller::startup::{self, launch_mode, LaunchMode};
use screen_bright_controller::{
    session::Session,
    ui::{
        popup_bounds, popup_min_size, popup_reopen_bounds, PopupArea, PopupRect, Status, UiSession,
    },
};
use std::sync::Mutex;
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
#[derive(Default)]
struct PopupPlacement {
    initialized: bool,
    areas: Vec<PopupArea>,
    selected_area: Option<PopupArea>,
    reconciling: bool,
}
impl PopupPlacement {
    fn needs_reconcile(&self, areas: &[PopupArea], selected: &PopupArea, force: bool) -> bool {
        !self.reconciling
            && self.initialized
            && (force || self.areas != areas || self.selected_area.as_ref() != Some(selected))
    }
}
type GeometryState = Mutex<PopupPlacement>;
const TRAY_ID: &str = "screen-bright-controller";
fn tray_tooltip(attention: bool) -> &'static str {
    if attention {
        "Screen Bright Controller · attention needed"
    } else {
        "Screen Bright Controller"
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
        let attention = handle.state::<State>().lock().map_or(true, |s| {
            s.as_ref().map_or(true, UiSession::needs_attention)
        });
        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(tray_tooltip(attention)));
        }
    });
}
fn restore_app(state: &State, app: &tauri::AppHandle) -> Result<(), String> {
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
    let result = restore_app(&state, &app);
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
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.hide();
    }
}
fn report(app: &tauri::AppHandle, error: String) {
    eprintln!("{error}");
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
/// Native window geometry is session memory. Only first show anchors to the tray.
/// Reopen/monitor/topology/DPI reconciliation never enters the display session or emits control events.
fn prepare_popup(
    app: &tauri::AppHandle,
    click: Option<tauri::PhysicalPosition<f64>>,
    force: bool,
) -> Result<(), String> {
    let areas: Vec<_> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|monitor| PopupArea {
            work: work_area(&monitor),
            scale: monitor.scale_factor(),
        })
        .collect();
    let initialized = {
        let placement = app.state::<GeometryState>();
        let guard = placement.lock().map_err(|_| "popup geometry lock failed")?;
        if guard.reconciling || (click.is_none() && !guard.initialized) {
            return Ok(());
        }
        guard.initialized
    }; // Never hold this lock while window setters may dispatch native events.
    let w = app.get_webview_window("popup").ok_or("popup unavailable")?;
    let position = w.outer_position().map_err(|e| e.to_string())?;
    let size = w.inner_size().map_err(|e| e.to_string())?;
    let current = if initialized {
        (position.x, position.y, size.width, size.height)
    } else {
        let click = click.ok_or("first popup requires tray position")?;
        let monitor = app
            .monitor_from_point(click.x, click.y)
            .map_err(|e| e.to_string())?
            .ok_or("tray monitor unavailable")?;
        popup_bounds(
            work_area(&monitor),
            (click.x, click.y),
            monitor.scale_factor(),
        )
    };
    let (index, (x, y, width, height)) =
        popup_reopen_bounds(&areas, current).ok_or("popup work area unavailable")?;
    let area = &areas[index];
    {
        let placement = app.state::<GeometryState>();
        let mut guard = placement.lock().map_err(|_| "popup geometry lock failed")?;
        if click.is_none() && !guard.needs_reconcile(&areas, area, force) {
            return Ok(());
        }
        // Setters can dispatch Moved/DPI events synchronously. Suppress reentry,
        // but release the mutex before any native setter to avoid deadlocks.
        guard.reconciling = true;
    }
    let result = (|| {
        let (min_w, min_h) = popup_min_size(area);
        // Clear old constraints before applying smaller work areas / new DPI minimums.
        w.set_min_size(None::<tauri::PhysicalSize<u32>>)
            .map_err(|e| e.to_string())?;
        w.set_max_size(None::<tauri::PhysicalSize<u32>>)
            .map_err(|e| e.to_string())?;
        // Physical constraints are converted using the window's current DPI by the runtime.
        // Move first, then bind constraints and re-read size after any native DPI adjustment.
        if position.x != x || position.y != y {
            w.set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        }
        w.set_min_size(Some(tauri::PhysicalSize::new(min_w, min_h)))
            .map_err(|e| e.to_string())?;
        w.set_max_size(Some(tauri::PhysicalSize::new(area.work.2, area.work.3)))
            .map_err(|e| e.to_string())?;
        let actual_size = w.inner_size().map_err(|e| e.to_string())?;
        if actual_size.width != width || actual_size.height != height {
            w.set_size(tauri::PhysicalSize::new(width, height))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    let placement = app.state::<GeometryState>();
    let mut guard = placement.lock().map_err(|_| "popup geometry lock failed")?;
    guard.reconciling = false;
    if result.is_ok() {
        guard.initialized = true;
        guard.selected_area = Some(area.clone());
        guard.areas = areas;
    }
    result
}
fn show_popup(
    app: &tauri::AppHandle,
    position: tauri::PhysicalPosition<f64>,
) -> Result<(), String> {
    prepare_popup(app, Some(position), false)?;
    let w = app.get_webview_window("popup").ok_or("popup unavailable")?;
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
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .manage(GeometryState::default())
        .manage(Mutex::new(Err::<UiSession, String>(
            "starting read-only snapshot".into(),
        )))
        .invoke_handler(tauri::generate_handler![
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
            *app.state::<State>().lock().unwrap() = session.and_then(UiSession::new);
            let worker = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_error = String::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let handle = worker.clone();
                    let _ = worker.run_on_main_thread(move || {
                        if let Err(error) = prepare_popup(&handle, None, false) {
                            eprintln!("Popup geometry: {error}");
                        }
                    });
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
            let quit_item = MenuItem::with_id(app, "quit", "Restore and quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&restore_item, &quit_item])?;
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
                        if app
                            .get_webview_window("popup")
                            .is_some_and(|w| w.is_visible().unwrap_or(false))
                        {
                            if let Some(w) = app.get_webview_window("popup") {
                                let _ = w.hide();
                            }
                        } else if let Err(e) = show_popup(app, position) {
                            report(app, e);
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "restore" => {
                        let result = restore_app(&app.state::<State>(), app);
                        if let Err(e) = result {
                            report(app, e);
                        }
                        let _ = app.emit("state-changed", ());
                    }
                    // ExitRequested restores once and cancels the exit if that fails.
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            refresh_attention(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "popup" {
                return;
            }
            match event {
                // Window events already arrive on the main thread.
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    let force = matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. });
                    if let Err(error) = prepare_popup(window.app_handle(), None, force) {
                        eprintln!("Popup geometry: {error}");
                    }
                }
                // Only explicit close hides the popup; focus loss does not.
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri application failed to initialize");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if let Err(e) = restore_app(&app.state::<State>(), app) {
                api.prevent_exit();
                report(app, e)
            }
        }
    });
}
#[cfg(test)]
mod popup_policy_tests {
    use super::*;
    #[test]
    fn popup_anchors_once_and_reconciles_topology_or_explicit_reopen() {
        let area = PopupArea {
            work: (0, 0, 1920, 1040),
            scale: 1.0,
        };
        let mut placement = PopupPlacement::default();
        assert!(!placement.needs_reconcile(std::slice::from_ref(&area), &area, false));
        placement.initialized = true;
        placement.areas = vec![area.clone()];
        placement.selected_area = Some(area.clone());
        assert!(!placement.needs_reconcile(std::slice::from_ref(&area), &area, false));
        assert!(placement.needs_reconcile(std::slice::from_ref(&area), &area, true));
        let moved_area = PopupArea {
            work: (-1920, 0, 1920, 1040),
            scale: 1.5,
        };
        assert!(placement.needs_reconcile(std::slice::from_ref(&moved_area), &moved_area, false));
        assert!(!placement.needs_reconcile(std::slice::from_ref(&area), &area, false));
    }
    #[test]
    fn moving_between_existing_same_dpi_work_areas_rebinds_constraints_once() {
        let areas = vec![
            PopupArea {
                work: (0, 0, 1920, 1040),
                scale: 1.0,
            },
            PopupArea {
                work: (1920, 0, 1280, 720),
                scale: 1.0,
            },
        ];
        let mut placement = PopupPlacement {
            initialized: true,
            areas: areas.clone(),
            selected_area: Some(areas[0].clone()),
            ..Default::default()
        };
        let (first, _) = popup_reopen_bounds(&areas, (100, 100, 720, 600)).unwrap();
        let (same, _) = popup_reopen_bounds(&areas, (200, 150, 720, 600)).unwrap();
        assert_eq!(first, same);
        assert!(!placement.needs_reconcile(&areas, &areas[same], false));
        let (next, bounds) = popup_reopen_bounds(&areas, (2200, 150, 1500, 900)).unwrap();
        assert_ne!(first, next);
        assert_eq!(areas[first].scale, areas[next].scale);
        assert_eq!(bounds, (1920, 0, 1280, 720));
        assert!(
            placement.needs_reconcile(&areas, &areas[next], false),
            "an existing same-DPI destination with different work-area dimensions needs new constraints"
        );
        placement.selected_area = Some(areas[next].clone());
        // A repeated native Moved event / periodic poll must not snap same-monitor dragging.
        let (same, clamped) = popup_reopen_bounds(&areas, (2250, 650, 720, 600)).unwrap();
        assert_eq!(same, next);
        assert_eq!(clamped, (2250, 120, 720, 600));
        assert!(!placement.needs_reconcile(&areas, &areas[same], false));
        assert!(placement.needs_reconcile(&areas, &areas[first], false));
        placement.reconciling = true;
        assert!(!placement.needs_reconcile(&areas, &areas[first], false));
        assert!(!placement.needs_reconcile(&areas, &areas[first], true));
        placement.reconciling = false;
        assert!(placement.needs_reconcile(&areas, &areas[first], false));
    }
}
