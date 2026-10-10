//! Backend-owned ephemeral controls; live_control is the intentional user-gesture write boundary.
use crate::{
    controller::{detached_only, Outcome},
    dimming_percent,
    session::Session,
};
use serde::Serialize;
use std::collections::BTreeMap;
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
}
impl UiSession {
    pub fn new(session: Session) -> Result<Self, String> {
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
        Ok(Self {
            session,
            controls,
            generation: 0,
            revision: 0,
            live_blocked: false,
            recovery: None,
            outcomes: vec![],
            message: "No changes applied.".into(),
            notice: None,
        })
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
        Err(e)
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        if let Err(e) = self.session.heartbeat() {
            self.recovery = Some(self.failure(&e, Recovery::Safety));
            self.message = format!("Safety stop: {e}. Restore before retrying.");
            self.live_blocked = true;
            self.generation += 1;
            self.revision += 1;
            return Err(e);
        }
        Ok(())
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
            self.session.reset_baseline(&id)?;
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
                self.message="Saved original gamma restored for attempted targets only. Visible effect unverified.".into();
                Ok(())
            }
            Err(e) => {
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
