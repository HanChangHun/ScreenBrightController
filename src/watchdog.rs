use crate::{
    controller::{Driver, Mock, Monitor},
    guard::Guard,
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
    time::{Duration, Instant},
};
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", deny_unknown_fields)]
pub enum Request {
    Hello { token: String },
    Arm { id: String, seconds: u64 },
    Continuous { id: String, lease: u64 },
    Renew { id: String, lease: u64 },
    Restore { id: String },
    Status,
    Quit,
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
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, ms: u32) -> u32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(
        algorithm: *mut std::ffi::c_void,
        buffer: *mut u8,
        length: u32,
        flags: u32,
    ) -> i32;
}
fn token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 32, 2) } != 0 {
        return Err("OS random generator failed".into());
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
struct Parent(*mut std::ffi::c_void);
impl Parent {
    fn open(pid: u32) -> Result<Self, String> {
        let h = unsafe { OpenProcess(0x00100000, 0, pid) };
        if h.is_null() {
            Err("cannot watch parent process".into())
        } else {
            Ok(Self(h))
        }
    }
    fn alive(&self) -> bool {
        (unsafe { WaitForSingleObject(self.0, 0) }) == 258
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
        restore_errors: g.errors.clone(),
        mock_values: g.driver.values(),
    }
}
pub fn serve(mock: bool) -> Result<(), String> {
    let secret = std::env::var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_TOKEN")
        .map_err(|_| "watchdog must be launched by application with owned pipes")?;
    let pid = std::env::var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT")
        .map_err(|_| "missing parent")?
        .parse::<u32>()
        .map_err(|_| "invalid parent")?;
    std::env::remove_var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_TOKEN");
    std::env::remove_var("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT");
    let parent = Parent::open(pid)?;
    if mock {
        serve_backend(Mock::default(), secret, parent)
    } else {
        serve_backend(Native, secret, parent)
    }
}
fn serve_backend<D: GuardBackend>(driver: D, secret: String, parent: Parent) -> Result<(), String> {
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
    match serde_json::from_str::<Request>(&hello).map_err(|e| e.to_string())? {
        Request::Hello { token } if token == secret => {}
        _ => return Err("invalid owned-pipe handshake".into()),
    }
    let mut g = Guard::new(driver)?;
    respond(&response(&g, None, true))?;
    let clock = Instant::now();
    let mut quitting = false;
    let mut disconnected = false;
    let mut death_at = None;
    loop {
        let now = clock.elapsed().as_millis() as u64;
        let alive = parent.alive() && !disconnected;
        if !alive || quitting {
            death_at.get_or_insert(now);
        }
        g.tick(now, alive && !quitting);
        if (!alive || quitting) && g.armed.is_empty() {
            let _ = respond(&response(&g, None, false));
            return Ok(());
        }
        if death_at.is_some_and(|t| now - t > 60000) {
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
                    Ok(Request::Arm { id, seconds }) if !quitting => {
                        let result = g.arm(&id, seconds, clock.elapsed().as_millis() as u64);
                        if result.is_ok() {
                            g.driver.simulate(&id);
                        }
                        result
                    }
                    Ok(Request::Continuous { id, lease }) if !quitting => {
                        let result =
                            g.arm_continuous(&id, lease, clock.elapsed().as_millis() as u64);
                        if result.is_ok() {
                            g.driver.simulate(&id);
                        }
                        result
                    }
                    Ok(Request::Renew { id, lease }) if !quitting => {
                        g.renew(&id, lease, clock.elapsed().as_millis() as u64)
                    }
                    Ok(Request::Restore { id }) => g.restore(&id),
                    Ok(Request::Status) => Ok(()),
                    Ok(Request::Quit) => {
                        quitting = true;
                        Ok(())
                    }
                    Ok(_) => Err("invalid state or command".into()),
                    Err(e) => Err(format!("invalid protocol: {e}")),
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
        let secret = token()?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = Command::new(exe)
            .arg(if mock {
                "--watchdog-mock"
            } else {
                "--watchdog"
            })
            .env("SCREEN_BRIGHT_CONTROLLER_WATCHDOG_TOKEN", &secret)
            .env(
                "SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT",
                std::process::id().to_string(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
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
        let response = link.request(Request::Hello { token: secret })?;
        link.monitors = response.monitors;
        Ok(link)
    }
    pub fn request(&mut self, request: Request) -> Result<Response, String> {
        if self.child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("watchdog exited; no gamma writes allowed".into());
        }
        let input = self.input.as_mut().ok_or("watchdog disconnected")?;
        serde_json::to_writer(&mut *input, &request).map_err(|e| e.to_string())?;
        writeln!(input).map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let response = self
            .output
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "watchdog response timeout; preview remains protected")??;
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
    fn arm(&mut self, id: &str, seconds: u64) -> Result<(), String> {
        self.link.request(Request::Arm {
            id: id.into(),
            seconds,
        })?;
        Ok(())
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
    fn restore(&mut self, id: &str, _original: &Ramp) -> Result<(), String> {
        self.link.request(Request::Restore { id: id.into() })?;
        Ok(())
    }
}
