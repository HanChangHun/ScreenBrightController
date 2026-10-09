#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use screen_bright_controller::startup::{self, launch_mode, LaunchMode};
use screen_bright_controller::{
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
fn status(state: tauri::State<'_, State>) -> Result<serde_json::Value, String> {
    with_session(&state, UiSession::status)
}
fn with_attention<T>(
    state: &State,
    f: impl FnOnce(&mut UiSession) -> Result<T, String>,
    publish: impl FnOnce(bool),
) -> Result<T, String> {
    let mut guard = state.lock().map_err(|_| "state lock failed")?;
    let result = match guard.as_mut() {
        Ok(session) => f(session),
        Err(e) => Err(e.clone()),
    };
    // Publish while still holding the state lock: Restore cannot overtake this update.
    publish(guard.as_ref().map_or(true, UiSession::needs_attention));
    result
}
fn publish_attention(app: &tauri::AppHandle, attention: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tray_tooltip(attention)));
    }
}
fn refresh_attention(app: &tauri::AppHandle) {
    let handle = app.clone();
    // Queue only AFTER releasing the operation lock. Tray setters synchronously
    // marshal to the main thread; a worker holding this lock could deadlock Restore.
    // Read authoritative state and publish together ON the main thread instead.
    let _ = app.run_on_main_thread(move || {
        let _ = with_attention(
            &handle.state::<State>(),
            |_| Ok(()),
            |attention| {
                publish_attention(&handle, attention);
            },
        );
    });
}
fn restore_app(state: &State, app: &tauri::AppHandle) -> Result<(), String> {
    let result = restore_handle(state, |_| {});
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
) -> Result<serde_json::Value, String> {
    let result = with_session(&state, |s| {
        s.live_control(&id, dim, enabled, generation)?;
        s.status()
    });
    refresh_attention(&app);
    let _ = app.emit("state-changed", ());
    result
}
#[tauri::command]
fn restore(
    state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let result = restore_app(&state, &app);
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

fn restore_handle(state: &State, publish: impl FnOnce(bool)) -> Result<(), String> {
    let mut guard = state.lock().map_err(|_| "state lock failed")?;
    let result = match guard.as_mut() {
        Ok(s) => s.restore(),
        Err(_) => Ok(()),
    };
    publish(guard.as_ref().map_or(true, UiSession::needs_attention));
    result
}

fn report(app: &tauri::AppHandle, error: String) {
    eprintln!("{error}");
    // Errors may be obsolete or unrelated to display recovery. Re-read current state
    // and publish under its lock rather than letting a delayed report undo Restore.
    refresh_attention(app);
    let _ = app.emit("display-error", error);
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
        if let Err(e) = screen_bright_controller::cli::run(&args) {
            eprintln!("{e}");
            std::process::exit(1)
        }
        return;
    }
    let demo = mode == LaunchMode::Demo;
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .manage(Mutex::new(Err::<UiSession, String>(
            "starting read-only snapshot".into(),
        )))
        .invoke_handler(tauri::generate_handler![
            get_autostart,
            set_autostart,
            status,
            live_control,
            restore,
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
            let pixels = include_bytes!("../icons/tray.rgba").to_vec();
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
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
                    "quit" => match restore_app(&app.state::<State>(), app) {
                        Ok(()) => app.exit(0),
                        Err(e) => report(app, e),
                    },
                    _ => {}
                })
                .build(app)?;

            refresh_attention(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "popup" && popup_should_hide(event) {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri application failed to initialize");
    app.run(|app, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            if let Err(e) = restore_app(&app.state::<State>(), app) {
                api.prevent_exit();
                report(app, e)
            }
        }
        tauri::RunEvent::Exit => {
            let _ = restore_app(&app.state::<State>(), app);
        }
        _ => {}
    });
}
#[cfg(test)]
mod popup_policy_tests {
    use super::*;
    #[test]
    fn stale_rejection_after_restore_does_not_mark_attention() {
        let state = Mutex::new(Ok(UiSession::new(Session::demo().unwrap()).unwrap()));
        with_session(&state, UiSession::restore).unwrap();
        let mut attention = true;
        let result = with_attention(
            &state,
            |s| s.live_control("master", 27, true, 0),
            |value| attention = value,
        );
        assert!(result.is_err());
        assert!(!attention, "obsolete cancellation is not current recovery");
    }
    #[test]
    fn restore_publishes_attention_under_state_lock() {
        let state = Mutex::new(Ok(UiSession::new(Session::demo().unwrap()).unwrap()));
        restore_handle(&state, |attention| {
            assert!(!attention);
            assert!(
                state.try_lock().is_err(),
                "Restore publication must retain the state lock"
            );
        })
        .unwrap();
    }
    #[test]
    fn blocked_recovery_survives_successful_heartbeat_and_clears_on_restore_and_fresh_request() {
        let state = Mutex::new(Ok(UiSession::new(Session::demo().unwrap()).unwrap()));
        let mut attention = false;
        with_attention(
            &state,
            |s| s.live_control("master", 35, true, 0),
            |v| attention = v,
        )
        .unwrap();
        assert!(!attention);
        with_session(&state, |s| {
            if let Session::Demo(c, _) = &mut s.session {
                c.driver.ignored_set = true;
            }
            Ok(())
        })
        .unwrap();
        assert!(with_attention(
            &state,
            |s| s.live_control("master", 52, true, 0),
            |v| attention = v
        )
        .is_err());
        assert!(attention);
        with_attention(&state, UiSession::heartbeat, |v| attention = v).unwrap();
        assert!(
            attention,
            "successful heartbeat cannot clear blocked recovery"
        );
        with_session(&state, |s| {
            if let Session::Demo(c, _) = &mut s.session {
                c.driver.fail_restore = true;
            }
            Ok(())
        })
        .unwrap();
        assert!(restore_handle(&state, |v| attention = v).is_err());
        assert!(attention);
        with_session(&state, |s| {
            if let Session::Demo(c, _) = &mut s.session {
                c.driver.fail_restore = false;
                c.driver.ignored_set = false;
            }
            Ok(())
        })
        .unwrap();
        restore_handle(&state, |v| attention = v).unwrap();
        assert!(!attention);
        // A delayed worker report reads current recovery, not the old error.
        with_attention(&state, |_| Ok(()), |v| attention = v).unwrap();
        assert!(!attention);
        attention = true; // Model stale presentation; fresh success re-publishes authoritative false.
        assert!(attention);
        with_attention(
            &state,
            |s| s.live_control("master", 27, true, s.generation),
            |v| attention = v,
        )
        .unwrap();
        assert!(!attention);
        assert!(!with_session(&state, |s| Ok(s.needs_attention())).unwrap());
    }
    #[test]
    fn attention_publication_prevents_restore_overtaking_live_completion() {
        use std::sync::{mpsc, Arc};
        let state = Arc::new(Mutex::new(Ok(
            UiSession::new(Session::demo().unwrap()).unwrap()
        )));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let writer_state = state.clone();
        let writer = std::thread::spawn(move || {
            with_attention(
                &writer_state,
                |s| s.live_control("master", 35, true, 0),
                |attention| {
                    assert!(!attention);
                    assert!(writer_state.try_lock().is_err());
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        });
        entered_rx.recv().unwrap();
        assert!(
            state.try_lock().is_err(),
            "state remains serialized until presentation completes"
        );
        let restore_state = state.clone();
        let restore = std::thread::spawn(move || {
            restore_handle(&restore_state, |attention| {
                assert!(!attention);
                assert!(restore_state.try_lock().is_err());
            })
            .unwrap();
        });
        release_tx.send(()).unwrap();
        writer.join().unwrap();
        restore.join().unwrap();
        let final_state = with_session(&state, UiSession::status).unwrap();
        assert_eq!(final_state["generation"], 1);
        assert_eq!(final_state["controls"]["master"]["dim"], 0);
    }
    #[test]
    fn delayed_attention_publication_reads_post_restore_state() {
        let state = Mutex::new(Ok(UiSession::new(Session::demo().unwrap()).unwrap()));
        with_session(&state, |s| {
            if let Session::Demo(c, _) = &mut s.session {
                c.driver.ignored_set = true;
            }
            s.live_control("master", 35, true, 0)
        })
        .unwrap_err();
        assert!(with_session(&state, |s| Ok(s.needs_attention())).unwrap());
        // Model the main-thread task queued by a failed live request/worker report.
        // It must capture neither the request error nor an attention boolean.
        let delayed_publish = || {
            let mut attention = true;
            with_attention(
                &state,
                |_| Ok(()),
                |value| {
                    assert!(state.try_lock().is_err());
                    attention = value;
                },
            )
            .unwrap();
            attention
        };
        restore_handle(&state, |_| {}).unwrap();
        assert!(!delayed_publish());
    }
    #[test]
    fn tray_tooltip_distinguishes_attention_without_opening_another_window() {
        assert_eq!(super::tray_tooltip(false), "Screen Bright Controller");
        assert_eq!(
            super::tray_tooltip(true),
            "Screen Bright Controller · attention needed"
        );
    }
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
