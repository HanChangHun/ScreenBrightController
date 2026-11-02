use crate::{
    controller::{Driver, Mock, Monitor},
    guard::Guard,
    log::write as log,
    native::Native,
    Ramp,
};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
    System::{
        Threading::{OpenProcess, WaitForSingleObject, CREATE_NO_WINDOW, PROCESS_SYNCHRONIZE},
        WindowsProgramming::QueryUnbiasedInterruptTime,
    },
};
/// Milliseconds of awake time. Unlike `Instant` (QueryPerformanceCounter), this clock
/// stops during sleep and hibernation, so a lease does not lapse merely because the PC
/// slept; after resume the app still verifies the ramp before its next renewal.
fn awake_ms() -> u64 {
    let mut ticks = 0u64;
    unsafe { QueryUnbiasedInterruptTime(&mut ticks) };
    ticks / 10_000
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", deny_unknown_fields)]
pub enum Request {
    Hello,
    Continuous { id: String, lease: u64 },
    Renew { id: String, lease: u64 },
    Restore { id: String },
    Reset { id: String },
    Status,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub ok: bool,
    pub error: Option<String>,
    pub monitors: Vec<Monitor>,
    pub armed: Vec<String>,
    pub restore_errors: Vec<String>,
    pub mock_values: Option<BTreeMap<String, u16>>,
}
struct Parent(HANDLE);
impl Parent {
    fn open(pid: u32) -> Result<Self, String> {
        let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if h.is_null() {
            Err("cannot watch parent process".into())
        } else {
            Ok(Self(h))
        }
    }
    fn alive(&self) -> bool {
        (unsafe { WaitForSingleObject(self.0, 0) }) == WAIT_TIMEOUT
    }
}
impl Drop for Parent {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn respond(response: &Response) -> Result<(), String> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, response).map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
pub trait GuardBackend: Driver {
    fn values(&self) -> Option<BTreeMap<String, u16>> {
        None
    }
    fn simulate(&mut self, _id: &str) {}
}
impl GuardBackend for Native {}
impl GuardBackend for Mock {
    fn values(&self) -> Option<BTreeMap<String, u16>> {
        Some(
            self.current
                .iter()
                .map(|(k, v)| (k.clone(), v[0][0]))
                .collect(),
        )
    }
    fn simulate(&mut self, id: &str) {
        let _ = self.set(id, &vec![vec![100; 256]; 3]);
    }
}
fn response<D: GuardBackend>(g: &Guard<D>, error: Option<String>, snapshot: bool) -> Response {
    Response {
        ok: error.is_none(),
        error,
        monitors: if snapshot { g.monitors.clone() } else { vec![] },
        armed: g.armed.keys().cloned().collect(),
        restore_errors: g
            .errors
            .iter()
            .map(|(id, e)| format!("{id}: {e}"))
            .collect(),
        mock_values: g.driver.values(),
    }
}
pub fn serve(mock: bool) -> Result<(), String> {
    let pid = std::env::var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT")
        .map_err(|_| "watchdog must be launched by the application")?
        .parse::<u32>()
        .map_err(|_| "invalid parent")?;
    std::env::remove_var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT");
    let parent = Parent::open(pid)?;
    if mock {
        serve_backend(Mock::default(), parent)
    } else {
        // Only the real watchdog logs; memory fixtures never touch the user's log.
        crate::log::init("watchdog.log");
        let result = serve_backend(Native, parent);
        if let Err(e) = &result {
            log(format!("watchdog stopped: {e}"));
        }
        result
    }
}
/// One guard tick, returning a log line per restored display and per new restore failure.
fn tick_events<D: Driver>(g: &mut Guard<D>, now: u64, alive: bool) -> Vec<String> {
    let before = g.armed.clone();
    let errors = g.errors.clone();
    g.tick(now, alive);
    let mut out = vec![];
    for (id, deadline) in &before {
        if !g.armed.contains_key(id) {
            let why = if errors.contains_key(id) {
                "retry succeeded"
            } else if !alive {
                "app gone"
            } else if *deadline <= now {
                "lease expired"
            } else {
                "restored"
            };
            out.push(format!("restored {id} ({why})"));
        } else if let Some(e) = g.errors.get(id).filter(|e| errors.get(id) != Some(*e)) {
            out.push(format!("restore {id} failed, retrying: {e}"));
        }
    }
    out
}
/// How long restoration keeps retrying after the parent is gone.
const RETRY_WINDOW_MS: u64 = 60_000;
/// Returns the new start of the retry window and whether the watchdog should give up.
/// The window opens at parent death (`death_at`) and restarts on every tick that waits for
/// an unplugged display, so a returned display gets the full window of restore retries.
fn retry_window(death_at: Option<u64>, now: u64, waiting_for_display: bool) -> (Option<u64>, bool) {
    match death_at {
        Some(_) if waiting_for_display => (Some(now), false),
        Some(start) => (Some(start), now.saturating_sub(start) > RETRY_WINDOW_MS),
        None => (None, false),
    }
}
fn serve_backend<D: GuardBackend>(driver: D, parent: Parent) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        loop {
            let mut line = String::new();
            match reader.by_ref().take(16385).read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if line.len() > 16384 {
                        break;
                    }
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let hello = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "watchdog handshake timeout")?;
    if !matches!(serde_json::from_str(&hello), Ok(Request::Hello)) {
        return Err("invalid handshake".into());
    }
    let mut g = Guard::new(driver)?;
    respond(&response(&g, None, true))?;
    let start = awake_ms();
    let clock = || awake_ms().saturating_sub(start);
    let mut disconnected = false;
    let mut death_at = None;
    loop {
        let now = clock();
        let alive = parent.alive() && !disconnected;
        if !alive && death_at.is_none() && !g.armed.is_empty() {
            let how = if parent.alive() {
                "closed its pipe"
            } else {
                "exited"
            };
            let ids: Vec<_> = g.armed.keys().cloned().collect();
            log(format!("app {how} while {} stayed armed", ids.join(", ")));
        }
        if !alive {
            death_at.get_or_insert(now);
        }
        for line in tick_events(&mut g, now, alive) {
            log(line);
        }
        if !alive && g.armed.is_empty() {
            let _ = respond(&response(&g, None, false));
            return Ok(());
        }
        // An unplugged display keeps its original until it returns; other failures stop after
        // 60s of retries, counted from parent death or from the display's return.
        let (window_start, give_up) = retry_window(death_at, now, g.waiting_for_display());
        death_at = window_start;
        if give_up {
            let _ = respond(&response(
                &g,
                Some("restoration failed after 60s retries".into()),
                false,
            ));
            return Err("restoration failed after 60s retries".into());
        }
        if disconnected {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                let result = match serde_json::from_str::<Request>(&line) {
                    Ok(Request::Continuous { id, lease }) => {
                        let result = g.arm_continuous(&id, lease, clock());
                        if result.is_ok() {
                            g.driver.simulate(&id);
                        }
                        log(match &result {
                            Ok(()) => format!("armed {id}"),
                            Err(e) => format!("arm {id} rejected: {e}"),
                        });
                        result
                    }
                    Ok(Request::Renew { id, lease }) => {
                        let result = g.renew(&id, lease, clock());
                        if let Err(e) = &result {
                            log(format!("renew {id} rejected: {e}"));
                        }
                        result
                    }
                    Ok(Request::Restore { id }) => {
                        let armed = g.armed.contains_key(&id);
                        let result = g.restore(&id);
                        match &result {
                            Ok(()) if armed => log(format!("restored {id} (app request)")),
                            Err(e) => log(format!("restore {id} failed (app request): {e}")),
                            Ok(()) => {}
                        }
                        result
                    }
                    Ok(Request::Reset { id }) => {
                        let result = g.reset(&id);
                        log(match &result {
                            Ok(()) => format!("reset {id} to the default ramp"),
                            Err(e) => format!("reset {id} rejected: {e}"),
                        });
                        result
                    }
                    Ok(Request::Status) => Ok(()),
                    Ok(Request::Hello) => Err("invalid state or command".into()),
                    Err(e) => {
                        log(format!("rejected request: invalid protocol: {e}"));
                        Err(format!("invalid protocol: {e}"))
                    }
                };
                // Arm is acknowledged only after the immutable original and deadline exist.
                if respond(&response(&g, result.err(), false)).is_err() {
                    disconnected = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => disconnected = true,
        }
    }
}
pub struct Link {
    pub child: Child,
    input: Option<ChildStdin>,
    output: Receiver<Result<Response, String>>,
    pub monitors: Vec<Monitor>,
}
impl Link {
    pub fn spawn(mock: bool) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = Command::new(exe)
            .arg(if mock {
                "--watchdog-mock"
            } else {
                "--watchdog"
            })
            .env(
                "SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT",
                std::process::id().to_string(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take();
        let stdout = child.stdout.take().ok_or("missing stdout")?;
        let (tx, output) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let result = line
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()));
                if tx.send(result).is_err() {
                    break;
                }
            }
        });
        let mut link = Self {
            child,
            input,
            output,
            monitors: vec![],
        };
        let response = link.request(Request::Hello)?;
        link.monitors = response.monitors;
        Ok(link)
    }
    pub fn request(&mut self, request: Request) -> Result<Response, String> {
        if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!(
                "watchdog exited ({status}); no gamma writes allowed"
            ));
        }
        // A reply that arrived after an earlier timeout must not answer this request.
        while self.output.try_recv().is_ok() {}
        let input = self.input.as_mut().ok_or("watchdog disconnected")?;
        serde_json::to_writer(&mut *input, &request).map_err(|e| e.to_string())?;
        writeln!(input).map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let response = self
            .output
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "watchdog response timeout; display remains protected")??;
        if !response.ok {
            return Err(response.error.unwrap_or("watchdog rejected request".into()));
        }
        Ok(response)
    }
    pub fn disconnect_and_wait(&mut self) -> Result<Response, String> {
        self.input.take();
        let response = self
            .output
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "watchdog exit timeout")??;
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("watchdog failed".into());
        }
        Ok(response)
    }
}
impl Drop for Link {
    fn drop(&mut self) {
        self.input.take(); /* NEVER kill the restoring process. EOF triggers restoration. */
    }
}
pub struct Real {
    pub link: Link,
}
impl Real {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            link: Link::spawn(false)?,
        })
    }
    pub fn status(&mut self) -> Result<Response, String> {
        self.link.request(Request::Status)
    }
}
impl Driver for Real {
    fn arm_continuous(&mut self, id: &str) -> Result<(), String> {
        self.link.request(Request::Continuous {
            id: id.into(),
            lease: 10,
        })?;
        Ok(())
    }
    fn renew(&mut self, id: &str) -> Result<(), String> {
        self.link.request(Request::Renew {
            id: id.into(),
            lease: 10,
        })?;
        Ok(())
    }
    fn snapshot(&mut self) -> Result<Vec<Monitor>, String> {
        Ok(self.link.monitors.clone())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String> {
        let status = self.link.request(Request::Status)?;
        if !status.armed.iter().any(|s| s == id) {
            return Err("watchdog not armed".into());
        }
        Native.set(id, ramp)
    }
    fn read(&mut self, id: &str) -> Result<Ramp, String> {
        Native.read(id)
    }
    fn reset(&mut self, id: &str, _linear: &Ramp) -> Result<(), String> {
        self.link.request(Request::Reset { id: id.into() })?;
        Ok(())
    }
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String> {
        match self.link.request(Request::Restore { id: id.into() }) {
            Ok(_) => Ok(()),
            // A dead watchdog cannot restore, but this process still owns the original.
            // Never respawn it: a fresh snapshot could capture the dimmed ramp as original.
            Err(_) if matches!(self.link.child.try_wait(), Ok(Some(_))) => {
                Native.restore(id, original)
            }
            Err(e) => Err(e),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// A link to a stand-in child; replies are injected through the returned sender.
    fn link(alive: bool) -> (Link, mpsc::Sender<Result<Response, String>>) {
        let mut child = Command::new("cmd")
            .args(["/c", if alive { "more" } else { "exit" }])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        if !alive {
            child.wait().unwrap();
        }
        let input = child.stdin.take();
        let (tx, output) = mpsc::channel();
        let link = Link {
            child,
            input,
            output,
            monitors: vec![],
        };
        (link, tx)
    }
    fn reply(error: Option<&str>) -> Result<Response, String> {
        Ok(Response {
            ok: error.is_none(),
            error: error.map(Into::into),
            monitors: vec![],
            armed: vec![],
            restore_errors: vec![],
            mock_values: None,
        })
    }
    fn reply_later(tx: &mpsc::Sender<Result<Response, String>>, error: Option<&'static str>) {
        let tx = tx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = tx.send(reply(error));
        });
    }
    #[test]
    fn exited_watchdog_falls_back_to_the_owned_original() {
        let (link, _tx) = link(false);
        let mut real = Real { link };
        // An unknown id is never written, so this exercises the fallback read-only.
        let error = real.restore("missing", &vec![vec![0; 256]; 3]).unwrap_err();
        assert!(error.contains("display disconnected"), "{error}");
    }
    #[test]
    fn live_watchdog_rejection_is_not_bypassed() {
        let (link, tx) = link(true);
        let mut real = Real { link };
        reply_later(&tx, Some("rejected"));
        let error = real.restore("missing", &vec![vec![0; 256]; 3]).unwrap_err();
        assert_eq!(error, "rejected");
    }
    #[test]
    fn late_reply_is_not_read_as_the_next_answer() {
        let (mut link, tx) = link(true);
        tx.send(reply(Some("late reply to a timed-out request")))
            .unwrap();
        reply_later(&tx, None);
        assert!(link.request(Request::Status).unwrap().ok);
    }
    #[test]
    fn awake_clock_advances_with_real_time_while_awake() {
        let start = awake_ms();
        std::thread::sleep(Duration::from_millis(300));
        let elapsed = awake_ms() - start;
        assert!((250..2000).contains(&elapsed), "{elapsed}ms");
    }
    #[test]
    fn tick_events_log_lease_restores_and_each_new_failure_once() {
        let mut g = Guard::new(Mock::default()).unwrap();
        g.arm_continuous("mock-1", 10, 0).unwrap();
        assert!(tick_events(&mut g, 5_000, true).is_empty());
        g.driver.fail_restore = true;
        let failed = tick_events(&mut g, 10_000, true);
        assert_eq!(failed.len(), 1, "{failed:?}");
        assert!(
            failed[0].contains("mock-1") && failed[0].contains("mock restore failure"),
            "{failed:?}"
        );
        assert!(
            tick_events(&mut g, 11_000, true).is_empty(),
            "an unchanged failure is logged once"
        );
        g.driver.fail_restore = false;
        assert_eq!(
            tick_events(&mut g, 12_000, true),
            ["restored mock-1 (retry succeeded)"]
        );
        g.arm_continuous("mock-2", 10, 13_000).unwrap();
        assert_eq!(
            tick_events(&mut g, 23_000, true),
            ["restored mock-2 (lease expired)"]
        );
        g.arm_continuous("mock-2", 10, 24_000).unwrap();
        assert_eq!(
            tick_events(&mut g, 24_100, false),
            ["restored mock-2 (app gone)"]
        );
    }
    #[test]
    fn returned_display_gets_a_full_retry_window_after_a_long_unplug() {
        /// One serve_backend iteration after the parent died, with memory mocks only.
        fn step(g: &mut Guard<Mock>, death_at: &mut Option<u64>, now: u64) -> &'static str {
            death_at.get_or_insert(now);
            g.tick(now, false);
            if g.armed.is_empty() {
                return "restored";
            }
            let (window_start, give_up) = retry_window(*death_at, now, g.waiting_for_display());
            *death_at = window_start;
            if give_up {
                "gave up"
            } else {
                "running"
            }
        }
        let mut g = Guard::new(Mock::default()).unwrap();
        g.arm_continuous("mock-1", 10, 0).unwrap();
        g.driver.set("mock-1", &vec![vec![100; 256]; 3]).unwrap();
        g.driver.detached = vec!["mock-1".into()];
        let mut death_at = None;
        // Unplugged for 90s after the parent died: the watchdog keeps waiting.
        for now in (1..90_001).step_by(100) {
            assert_eq!(step(&mut g, &mut death_at, now), "running", "at {now}ms");
        }
        assert!(g.waiting_for_display());
        // The display returns, but its first restore fails for another reason.
        g.driver.detached.clear();
        g.driver.fail_restore = true;
        let returned = 90_001;
        for now in (returned..returned + 60_000).step_by(100) {
            let state = step(&mut g, &mut death_at, now);
            assert_eq!(state, "running", "{}ms after the return", now - returned);
            assert!(!g.waiting_for_display() && g.errors.contains_key("mock-1"));
        }
        // The window does end 60s after the return if restoration keeps failing.
        assert!(retry_window(death_at, returned + 60_000, false).1);
        // A later retry succeeds and disarms the target with its original restored.
        g.driver.fail_restore = false;
        assert_eq!(step(&mut g, &mut death_at, returned + 60_000), "restored");
        assert!(g.armed.is_empty() && g.errors.is_empty());
        assert_eq!(g.driver.current["mock-1"][0][0], 40000);
    }
}
