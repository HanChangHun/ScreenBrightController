//! Backend-owned ephemeral controls; live_control is the intentional user-gesture write boundary.
use crate::{
    controller::{detached_only, Monitor, Outcome},
    dimming_percent,
    log::write as log,
    session::Session,
};
use serde::Serialize;
use std::collections::BTreeMap;
/// Whether a tray left-click opens the popup. Pressing the tray icon takes focus from a
/// visible popup, so focus loss may hide it just before the click arrives; a click within
/// 500 ms of such a hide is the closing click and must not reopen it.
pub fn tray_click_shows_popup(visible: bool, ms_since_focus_hide: Option<u64>) -> bool {
    !visible && ms_since_focus_hide.is_none_or(|ms| ms > 500)
}
/// Physical x, y, width, height.
pub type PopupRect = (i32, i32, u32, u32);
/// Reopening keeps the user's native geometry while some work area shows the popup's top strip.
pub fn popup_on_screen(work_areas: &[PopupRect], popup: PopupRect) -> bool {
    let (x, y, width, _) = popup;
    let (left, top) = (i64::from(x), i64::from(y));
    let (right, bottom) = (left + i64::from(width), top + 24);
    work_areas.iter().any(|&(ax, ay, aw, ah)| {
        let (ax, ay) = (i64::from(ax), i64::from(ay));
        left < ax + i64::from(aw) && right > ax && top < ay + i64::from(ah) && bottom > ay
    })
}
/// Logical pixels to physical pixels; an invalid scale factor counts as 1.
fn physical(logical: f64, scale: f64) -> u32 {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    (logical * scale).round() as u32
}
/// Coordinates are physical throughout; only the desired logical size/gap is DPI-scaled.
pub fn popup_bounds(work: PopupRect, click: (f64, f64), scale: f64) -> PopupRect {
    let (left, top, width, height) = work;
    let w = physical(430.0, scale).min(width);
    let h = physical(340.0, scale).min(height);
    let gap = physical(12.0, scale) as i32;
    let x = (click.0.round() as i32 - w as i32 - gap).clamp(left, left + width as i32 - w as i32);
    let y = (click.1.round() as i32 - h as i32 - gap).clamp(top, top + height as i32 - h as i32);
    (x, y, w, h)
}
/// Whole wheel detents only; invalid hover, injected input, a new generation or a
/// long pause discards a high-resolution wheel's partial detent.
#[derive(Default)]
pub struct TrayWheel {
    remainder: i32,
    generation: u64,
    last_ms: u64,
}
impl TrayWheel {
    pub fn steps(
        &mut self,
        delta: i32,
        over_icon: bool,
        injected: bool,
        generation: u64,
        now_ms: u64,
    ) -> i32 {
        if !over_icon
            || injected
            || generation != self.generation
            || now_ms.saturating_sub(self.last_ms) > 500
        {
            self.remainder = 0;
        }
        self.generation = generation;
        self.last_ms = now_ms;
        if !over_icon || injected {
            return 0;
        }
        self.remainder = self.remainder.saturating_add(delta);
        let steps = self.remainder / 120;
        self.remainder %= 120;
        steps
    }
}
#[derive(Clone, Serialize)]
pub struct Control {
    pub dim: i64,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    Apply,
    Safety,
    Restore,
    /// Only unplugged displays block restoration; the watchdog keeps them armed.
    Detached,
}
#[derive(Serialize)]
pub struct MonitorStatus {
    pub id: String,
    pub name: String,
    pub supported: bool,
    pub error: Option<String>,
    pub dimmed: bool,
}
/// Exactly what the popup reads.
#[derive(Serialize)]
pub struct Status {
    pub monitors: Vec<MonitorStatus>,
    pub restore_errors: Vec<String>,
    pub controls: BTreeMap<String, Control>,
    pub generation: u64,
    pub revision: u64,
    pub live_blocked: bool,
    pub recovery: Option<Recovery>,
    pub outcomes: Vec<Outcome>,
    pub message: String,
    pub notice: Option<String>,
}
pub struct UiSession {
    pub session: Session,
    pub controls: BTreeMap<String, Control>,
    pub generation: u64,
    pub revision: u64,
    live_blocked: bool,
    recovery: Option<Recovery>,
    pub outcomes: Vec<Outcome>,
    pub message: String,
    /// Non-blocking: a rejected update left the previous verified level in place.
    notice: Option<String>,
    /// Last control levels written to the log, so a slider drag logs one settled line.
    logged_level: String,
}
/// Display IDs, names and saved-original tops, for the log.
fn describe(monitors: &[Monitor]) -> String {
    monitors
        .iter()
        .map(|m| match &m.original {
            Some(r) => format!(
                "{} \"{}\" top {}{}",
                m.id,
                m.name,
                crate::tops(r),
                if crate::looks_dimmed(r) {
                    " (looks dimmed)"
                } else {
                    ""
                }
            ),
            None => format!(
                "{} \"{}\" unreadable: {}",
                m.id,
                m.name,
                m.error.as_deref().unwrap_or("unknown")
            ),
        })
        .collect::<Vec<_>>()
        .join("; ")
}
/// API and readback per display, for the log.
fn outcomes_text(rows: &[Outcome]) -> String {
    rows.iter()
        .map(|r| {
            let readback = match (r.readback_matches, &r.readback_error) {
                (Some(true), _) => "matched".to_string(),
                (Some(false), _) => "differs".into(),
                (None, Some(e)) => format!("failed ({e})"),
                (None, None) => "unavailable".into(),
            };
            format!(
                "{} API {} readback {readback}{}",
                r.id,
                if r.api_success { "ok" } else { "rejected" },
                if r.kept_previous {
                    " kept previous"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
impl UiSession {
    pub fn new(session: Session) -> Result<Self, String> {
        log(format!("session started: {}", describe(session.monitors())));
        let mut controls = BTreeMap::from([(
            "master".into(),
            Control {
                dim: 0,
                enabled: true,
            },
        )]);
        for monitor in session.monitors() {
            controls.insert(
                monitor.id.clone(),
                Control {
                    dim: 0,
                    enabled: monitor.original.is_some(),
                },
            );
        }
        let mut ui = Self {
            session,
            controls,
            generation: 0,
            revision: 0,
            live_blocked: false,
            recovery: None,
            outcomes: vec![],
            message: "No changes applied.".into(),
            notice: None,
            logged_level: String::new(),
        };
        ui.logged_level = ui.level();
        Ok(ui)
    }
    /// Control levels as `id=dim`, with ` off` for an excluded display.
    fn level(&self) -> String {
        self.controls
            .iter()
            .map(|(id, c)| format!("{id}={}{}", c.dim, if c.enabled { "" } else { " off" }))
            .collect::<Vec<_>>()
            .join(", ")
    }
    /// Authoritative recovery state; read-only and independent of request errors.
    pub fn needs_attention(&self) -> bool {
        self.live_blocked || self.recovery.is_some()
    }
    /// Quit may leave the remaining unplugged displays to the running watchdog.
    pub fn detached(&self) -> bool {
        self.recovery == Some(Recovery::Detached)
    }
    fn failure(&mut self, error: &str, otherwise: Recovery) -> Recovery {
        if detached_only(error) && self.session.watchdog_running() {
            Recovery::Detached
        } else {
            otherwise
        }
    }
    pub fn status(&mut self) -> Result<Status, String> {
        Ok(Status {
            restore_errors: self.session.restore_errors()?,
            monitors: self
                .session
                .monitors()
                .iter()
                .map(|m| MonitorStatus {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    supported: m.original.is_some(),
                    error: m.error.clone(),
                    dimmed: m.original.as_ref().is_some_and(crate::looks_dimmed),
                })
                .collect(),
            controls: self.controls.clone(),
            generation: self.generation,
            revision: self.revision,
            live_blocked: self.live_blocked,
            recovery: self.recovery,
            outcomes: self.outcomes.clone(),
            message: self.message.clone(),
            notice: self.notice.clone(),
        })
    }
    /// Native physical wheel gesture; never changes independent mode or exclusions.
    /// A changed level invalidates already queued popup intents before applying.
    pub fn tray_wheel(&mut self, steps: i32, generation: u64) -> Result<(), String> {
        if generation != self.generation || self.live_blocked {
            return Err("Live request cancelled; Restore before retrying.".into());
        }
        let master = &self.controls["master"];
        if !master.enabled || steps == 0 {
            return Ok(());
        }
        // Match the popup slider: wheel up increases dimming, wheel down decreases it.
        let dim = (master.dim + i64::from(steps) * 5).clamp(0, 90);
        if dim == master.dim {
            return Ok(());
        }
        self.generation += 1;
        self.live_control("master", dim, true, self.generation)
    }
    /// Only intentional UI gestures use this command. Generation fences all pre-Restore intents.
    pub fn live_control(
        &mut self,
        id: &str,
        dim: i64,
        enabled: bool,
        generation: u64,
    ) -> Result<(), String> {
        dimming_percent(dim)?;
        if generation != self.generation || self.live_blocked {
            return Err("Live request cancelled; Restore before retrying.".into());
        }
        let previous = self.controls.get(id).cloned().ok_or("unknown control")?;
        let before = self.controls.clone();
        self.controls.insert(id.into(), Control { dim, enabled });
        self.revision += 1;
        self.notice = None;
        match self.apply() {
            Ok(true) => Ok(()),
            // Every refusing display still reads back its last verified ramp. Return the
            // controls to that state and drop requests queued on the rejected value.
            Ok(false) => {
                let rejected = self.outcomes.clone();
                self.controls = before;
                self.generation += 1;
                match self.apply() {
                    Ok(true) => {
                        log(format!(
                            "Windows rejected {dim} and kept {}: {}",
                            previous.dim,
                            outcomes_text(&rejected)
                        ));
                        self.outcomes = rejected;
                        self.notice = Some(if previous.dim == dim {
                            "Windows rejected this change and kept the previous setting.".into()
                        } else {
                            format!("Windows rejected {dim} and kept {}.", previous.dim)
                        });
                        Ok(())
                    }
                    Ok(false) => self.stop("returning to the previous level was rejected".into()),
                    Err(e) => self.stop(e),
                }
            }
            Err(e) => self.stop(e),
        }
    }
    fn stop(&mut self, e: String) -> Result<(), String> {
        self.live_blocked = true;
        self.recovery = Some(Recovery::Apply);
        self.generation += 1;
        self.message = format!("Live update stopped: {e}. Restore before retrying.");
        log(format!(
            "live update stopped, waiting for Restore: {e}; {}",
            outcomes_text(&self.outcomes)
        ));
        Err(e)
    }
    /// A safety stop restores every display itself and needs the user only when that
    /// fails; the stop then keeps retrying. Rejected or unverified writes (Apply) are
    /// never cleared without the user's Restore.
    pub fn heartbeat(&mut self) -> Result<(), String> {
        match self.recovery {
            None => match self.session.heartbeat() {
                Ok(()) => {
                    let level = self.level();
                    if level != self.logged_level {
                        log(format!("dimming set: {level}"));
                        self.logged_level = level;
                    }
                    Ok(())
                }
                Err(e) => self.safety_stop(e),
            },
            Some(Recovery::Apply) => self.session.heartbeat(),
            Some(kind) => self.retry_restore(kind),
        }
    }
    fn safety_stop(&mut self, e: String) -> Result<(), String> {
        // Fence intents queued on the old level in every window before restoring.
        self.generation += 1;
        self.revision += 1;
        let detail = match self
            .session
            .restore()
            .and_then(|()| self.session.all_restored())
        {
            Ok(true) => {
                self.turned_off(
                    "Display changed, so dimming was turned off. Dim again when ready.",
                    &e,
                );
                return Ok(());
            }
            Ok(false) => "the watchdog still reports a pending restoration".to_string(),
            Err(detail) => detail,
        };
        self.recovery = Some(self.failure(&detail, Recovery::Safety));
        self.message = format!(
            "Safety stop: {e}. Automatic restore failed: {detail}. Restore before retrying."
        );
        self.live_blocked = true;
        log(format!(
            "safety stop: {e}; automatic restore failed, retrying: {detail}"
        ));
        Err(e)
    }
    fn retry_restore(&mut self, kind: Recovery) -> Result<(), String> {
        let error = match self
            .session
            .restore()
            .and_then(|()| self.session.all_restored())
        {
            Ok(true) => {
                self.turned_off(
                    "Original brightness restored. Dim again when ready.",
                    "a retried restore succeeded",
                );
                return Ok(());
            }
            Ok(false) => "the watchdog still reports a pending restoration".to_string(),
            Err(e) => e,
        };
        let otherwise = if kind == Recovery::Safety {
            Recovery::Safety
        } else {
            Recovery::Restore
        };
        let recovery = self.failure(&error, otherwise);
        if self.recovery != Some(recovery) {
            self.recovery = Some(recovery);
            self.revision += 1;
            self.message = match recovery {
                Recovery::Detached => format!("Restore waits for an unplugged display: {error}. The watchdog restores it when it returns."),
                _ => format!("Restore failed: {error}. Keep the app open and retry."),
            };
            log(format!("restore still failing: {error}"));
        }
        Err(error)
    }
    /// Every display is back at its saved original: unlock the controls at 0.
    fn turned_off(&mut self, notice: &str, reason: &str) {
        self.live_blocked = false;
        self.recovery = None;
        for control in self.controls.values_mut() {
            control.dim = 0;
        }
        self.outcomes.clear();
        self.generation += 1;
        self.revision += 1;
        self.notice = Some(notice.into());
        self.message = format!("Dimming turned off automatically: {reason}.");
        self.logged_level = self.level();
        log(format!("dimming turned off automatically: {reason}"));
    }
    /// Ok(false): a display refused the new ramp and still reads back its last verified one.
    fn apply(&mut self) -> Result<bool, String> {
        let master = self.controls["master"].clone();
        let mut targets = vec![];
        for monitor in self.session.monitors() {
            let control = &self.controls[&monitor.id];
            let dim = if master.enabled {
                master.dim
            } else {
                control.dim
            };
            let percent = dimming_percent(dim)?;
            targets.push((
                monitor.id.clone(),
                if control.enabled && monitor.original.is_some() {
                    percent
                } else {
                    100
                },
            ));
        }
        self.outcomes.clear();
        for (id, percent) in targets {
            if percent == 100 {
                self.session.restore_target(&id)?;
                continue;
            }
            let rows = self.session.continuous(vec![id], percent)?;
            let failed: Vec<bool> = rows
                .iter()
                .filter(|r| !r.api_success || r.readback_matches != Some(true))
                .map(|r| r.kept_previous)
                .collect();
            self.outcomes.extend(rows);
            if failed.iter().any(|kept| !kept) {
                return Err("API rejected, readback failed or driver ignored dimming; attempted targets remain watchdog protected".into());
            }
            if !failed.is_empty() {
                return Ok(false);
            }
        }
        self.message="Continuous operation applied. Closing hides to tray; Restore or Quit returns saved originals. Native lease protection active.".into();
        Ok(true)
    }
    /// User-initiated only: give every display with a dimmed saved original the linear ramp.
    pub fn reset_baseline(&mut self) -> Result<(), String> {
        let ids: Vec<String> = self
            .session
            .monitors()
            .iter()
            .filter(|m| m.original.as_ref().is_some_and(crate::looks_dimmed))
            .map(|m| m.id.clone())
            .collect();
        self.revision += 1;
        for id in ids {
            if let Err(e) = self.session.reset_baseline(&id) {
                log(format!("reset to default ramp failed: {e}"));
                return Err(e);
            }
            log(format!("{id}: saved original reset to the default ramp"));
        }
        Ok(())
    }
    pub fn restore(&mut self) -> Result<(), String> {
        self.generation += 1;
        self.revision += 1;
        self.live_blocked = true;
        self.notice = None;
        match self.session.restore() {
            Ok(()) => {
                self.live_blocked = false;
                self.recovery = None;
                for control in self.controls.values_mut() {
                    control.dim = 0;
                }
                self.outcomes.clear();
                self.logged_level = self.level();
                log("Restore: saved originals restored");
                self.message="Saved original gamma restored for attempted targets only. Visible effect unverified.".into();
                Ok(())
            }
            Err(e) => {
                log(format!("Restore failed: {e}"));
                let recovery = self.failure(&e, Recovery::Restore);
                self.recovery = Some(recovery);
                self.message = if recovery == Recovery::Detached {
                    format!("Restore waits for an unplugged display: {e}. The watchdog restores it when it returns.")
                } else {
                    format!("Restore failed: {e}. Keep the app open and retry.")
                };
                Err(e)
            }
        }
    }
}
