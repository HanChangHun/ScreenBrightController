#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use gamma_dimmer::{
    session::Session,
    ui::{popup_bounds, UiSession},
};
use std::sync::Mutex;
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
fn set_control(
    id: String,
    dim: i64,
    enabled: bool,
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    with_session(&state, |s| s.set_control(&id, dim, enabled))?;
    let _ = app.emit("state-changed", ());
    Ok(())
}
#[tauri::command]
fn set_consent(
    consent: bool,
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    with_session(&state, |s| {
        s.consent = consent;
        Ok(())
    })?;
    let _ = app.emit("state-changed", ());
    Ok(())
}
#[tauri::command]
fn preview(state: tauri::State<'_, State>, app: tauri::AppHandle) -> Result<(), String> {
    let result = with_session(&state, UiSession::preview);
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn apply(
    mode: String,
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let result = with_session(&state, |s| s.apply(&mode));
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn restore(state: tauri::State<'_, State>, app: tauri::AppHandle) -> Result<(), String> {
    let result = restore_handle(&state);
    let _ = app.emit("state-changed", ());
    result
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
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !args.is_empty() && args[0] != "--demo" {
        if let Err(e) = gamma_dimmer::cli::run(&args) {
            eprintln!("{e}");
            std::process::exit(1)
        }
        return;
    }
    let demo = args.first().is_some_and(|s| s == "--demo");
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .manage(Mutex::new(Err::<UiSession, String>(
            "starting read-only snapshot".into(),
        )))
        .invoke_handler(tauri::generate_handler![
            status,
            set_control,
            set_consent,
            preview,
            apply,
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
                        if let Err(e) = show_popup(tray.app_handle(), position) {
                            report(tray.app_handle(), e);
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
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "popup" {
                match event {
                    tauri::WindowEvent::Focused(false) => {
                        let _ = window.hide();
                    }
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    _ => {}
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
