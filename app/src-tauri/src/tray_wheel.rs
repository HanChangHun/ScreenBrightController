//! Physical tray wheel input. The low-level hook never waits, writes gamma or
//! calls Tauri's main-thread-dispatched tray APIs. One bounded slot feeds one worker.
use crate::{refresh_attention, report, State, TRAY_ID};
use screen_bright_controller::ui::TrayWheel;
use std::{
    cell::RefCell,
    sync::mpsc,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetAncestor, GetClassNameW, GetCursorPos, GetMessageW,
        SetWindowsHookExW, UnhookWindowsHookEx, WindowFromPoint, GA_ROOT, LLMHF_INJECTED,
        LLMHF_LOWER_IL_INJECTED, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_MOUSEWHEEL,
    },
};
const FRESH: Duration = Duration::from_millis(200);
struct Gesture {
    x: i32,
    y: i32,
    delta: i32,
    generation: u64,
    at: Instant,
}
struct Hook {
    app: tauri::AppHandle,
    tx: mpsc::SyncSender<Gesture>,
}
thread_local! { static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) }; }
unsafe extern "system" fn mouse(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && message == WM_MOUSEWHEEL as usize && data != 0 {
        let input = unsafe { &*(data as *const MSLLHOOKSTRUCT) };
        if input.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) == 0 {
            HOOK.with(|slot| {
                let Ok(slot) = slot.try_borrow() else {
                    return;
                };
                let Some(hook) = slot.as_ref() else {
                    return;
                };
                let Some(state) = hook.app.try_state::<State>() else {
                    return;
                };
                let Ok(guard) = state.try_lock() else {
                    return;
                };
                let Ok(session) = guard.as_ref() else {
                    return;
                };
                if session.needs_attention() || !session.controls["master"].enabled {
                    return;
                }
                let _ = hook.tx.try_send(Gesture {
                    x: input.pt.x,
                    y: input.pt.y,
                    delta: (input.mouseData >> 16) as i16 as i32,
                    generation: session.generation,
                    at: Instant::now(),
                });
            });
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, message, data) }
}
// On the main thread, query the *current* shell icon rectangle, not a cached hover
// box. A taskbar/overflow window must actually be under both the event and cursor.
fn over_icon(app: &tauri::AppHandle, x: i32, y: i32) -> bool {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return false;
    };
    let Ok(Some(rect)) = tray.rect() else {
        return false;
    };
    let position = rect.position.to_physical::<i32>(1.0);
    let size = rect.size.to_physical::<u32>(1.0);
    let inside = |p: POINT| {
        let (dx, dy) = (
            i64::from(p.x) - i64::from(position.x),
            i64::from(p.y) - i64::from(position.y),
        );
        dx >= 0 && dy >= 0 && dx < i64::from(size.width) && dy < i64::from(size.height)
    };
    let shell = |p: POINT| unsafe {
        let root = GetAncestor(WindowFromPoint(p), GA_ROOT);
        let mut name = [0u16; 128];
        let n = GetClassNameW(root, name.as_mut_ptr(), name.len() as i32);
        if n <= 0 {
            return false;
        }
        matches!(
            String::from_utf16_lossy(&name[..n as usize]).as_str(),
            "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "NotifyIconOverflowWindow"
        )
    };
    let event = POINT { x, y };
    let mut current = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut current) != 0 })
        && inside(event)
        && inside(current)
        && shell(event)
        && shell(current)
}
pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel::<Gesture>(1);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let handle = app.clone();
    std::thread::Builder::new()
        .name("tray-wheel-hook".into())
        .spawn(move || {
            HOOK.with(|slot| *slot.borrow_mut() = Some(Hook { app: handle, tx }));
            let hook = unsafe {
                SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(mouse),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                )
            };
            if hook.is_null() {
                let _ = ready_tx.send(Err(format!(
                    "Tray wheel unavailable: {}",
                    std::io::Error::last_os_error()
                )));
                HOOK.with(|slot| *slot.borrow_mut() = None);
                return;
            }
            let _ = ready_tx.send(Ok(()));
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    DispatchMessageW(&msg);
                }
                UnhookWindowsHookEx(hook);
            }
            HOOK.with(|slot| *slot.borrow_mut() = None);
        })
        .map_err(|e| e.to_string())?;
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .map_err(|e| e.to_string())??;
    let worker = app.clone();
    std::thread::Builder::new()
        .name("tray-wheel-apply".into())
        .spawn(move || {
            let mut wheel = TrayWheel::default();
            let started = Instant::now();
            for gesture in rx {
                if gesture.at.elapsed() > FRESH {
                    wheel = TrayWheel::default();
                    continue;
                }
                let (answer, result) = mpsc::sync_channel(1);
                let handle = worker.clone();
                let (x, y) = (gesture.x, gesture.y);
                if worker
                    .run_on_main_thread(move || {
                        let _ = answer.try_send(over_icon(&handle, x, y));
                    })
                    .is_err()
                {
                    continue;
                }
                let over = result.recv_timeout(FRESH).unwrap_or(false);
                let steps = wheel.steps(
                    gesture.delta,
                    over,
                    false,
                    gesture.generation,
                    started.elapsed().as_millis() as u64,
                );
                if steps == 0 || gesture.at.elapsed() > FRESH {
                    continue;
                }
                let state = worker.state::<State>();
                // Drop rather than queue behind Restore/heartbeat or another live input.
                let Ok(mut guard) = state.try_lock() else {
                    continue;
                };
                let Ok(session) = guard.as_mut() else {
                    continue;
                };
                if session.generation != gesture.generation || gesture.at.elapsed() > FRESH {
                    continue;
                }
                let result = session.tray_wheel(steps, gesture.generation);
                drop(guard);
                refresh_attention(&worker);
                let _ = worker.emit("state-changed", ());
                if let Err(error) = result {
                    report(&worker, error);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
