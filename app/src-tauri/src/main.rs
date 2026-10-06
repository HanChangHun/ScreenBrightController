#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use gamma_dimmer::startup::{self, launch_mode, LaunchMode};
use gamma_dimmer::{
    session::Session,
    ui::{popup_bounds, UiSession},
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
fn status(state: tauri::State<'_, State>) -> Result<serde_json::Value, String> {
    with_session(&state, UiSession::status)
}
#[tauri::command]
fn live_control(
    id: String,
    dim: i64,
    enabled: bool,
    generation: u64,
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let result = with_session(&state, |s| {
        s.live_control(&id, dim, enabled, generation)?;
        s.status()
    });
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn restore(
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let result = restore_handle(&state);
    let _ = app.emit("state-changed", ());
    result?;
    with_session(&state, UiSession::status)
}
#[tauri::command]
fn hide_popup(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.hide();
    }
}
#[tauri::command]
fn open_main(app: tauri::AppHandle) {
    hide_popup(app.clone());
    show(&app);
}
fn restore_handle(state: &State) -> Result<(), String> {
    match state.lock().map_err(|_| "state lock failed")?.as_mut() {
        Ok(s) => s.restore(),
        Err(_) => Ok(()),
    }
}
fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn report(app: &tauri::AppHandle, error: String) {
    eprintln!("{error}");
    let _ = app.emit("gamma-error", error);
    show(app);
}
fn show_popup(
    app: &tauri::AppHandle,
    position: tauri::PhysicalPosition<f64>,
) -> Result<(), String> {
    let w = app.get_webview_window("popup").ok_or("popup unavailable")?;
    let monitor = app
        .monitor_from_point(position.x, position.y)
        .map_err(|e| e.to_string())?
        .ok_or("tray monitor unavailable")?;
    let work = monitor.work_area();
    let (x, y, width, height) = popup_bounds(
        (
            work.position.x,
            work.position.y,
            work.size.width,
            work.size.height,
        ),
        (position.x, position.y),
        monitor.scale_factor(),
    );
    w.set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    w.set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit("state-changed", ());
    Ok(())
}
fn popup_should_hide(event: &tauri::WindowEvent) -> bool {
    matches!(event, tauri::WindowEvent::CloseRequested { .. })
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
        if let Err(e) = gamma_dimmer::cli::run(&args) {
            eprintln!("{e}");
            std::process::exit(1)
        }
        return;
    }
    let demo = mode == LaunchMode::Demo;
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if launch_mode(args.get(1..).unwrap_or_default()) != LaunchMode::Tray {
                show(app);
            }
        }))
        .manage(Mutex::new(Err::<UiSession, String>(
            "starting read-only snapshot".into(),
        )))
        .invoke_handler(tauri::generate_handler![
            get_autostart,
            set_autostart,
            status,
            live_control,
            restore,
            hide_popup,
            open_main
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
                    match with_session(&worker.state::<State>(), UiSession::heartbeat) {
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
            let show_item = MenuItem::with_id(
                app,
                "show",
                "Open Screen Bright Controller",
                true,
                None::<&str>,
            )?;
            let restore_item = MenuItem::with_id(
                app,
                "restore",
                "Restore saved original gamma",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "Restore and quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &restore_item, &quit_item])?;
            let pixels = include_bytes!("../icons/tray.rgba").to_vec();
            TrayIconBuilder::new()
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("Screen Bright Controller · protected continuous dimming")
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
                    "show" => show(app),
                    "restore" => {
                        if let Err(e) = restore_handle(&app.state::<State>()) {
                            report(app, e);
                        }
                        let _ = app.emit("state-changed", ());
                    }
                    "quit" => match restore_handle(&app.state::<State>()) {
                        Ok(()) => app.exit(0),
                        Err(e) => report(app, e),
                    },
                    _ => {}
                })
                .build(app)?;
            if mode != LaunchMode::Tray {
                show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "popup" {
                if popup_should_hide(event) {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                    }
                    let _ = window.hide();
                }
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle();
                match with_session(&app.state::<State>(), UiSession::main_close) {
                    Ok(()) => {
                        let _ = window.hide();
                        let _ = app.emit("state-changed", ());
                    }
                    Err(e) => report(app, e),
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri application failed to initialize");
    app.run(|app, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            if let Err(e) = restore_handle(&app.state::<State>()) {
                api.prevent_exit();
                report(app, e)
            }
        }
        tauri::RunEvent::Exit => {
            let _ = restore_handle(&app.state::<State>());
        }
        _ => {}
    });
}
#[cfg(test)]
mod popup_policy_tests {
    #[test]
    fn focus_loss_does_not_hide_popup() {
        assert!(!super::popup_should_hide(&tauri::WindowEvent::Focused(
            false
        )));
        assert!(!super::popup_should_hide(&tauri::WindowEvent::Focused(
            true
        )));
    }
}
