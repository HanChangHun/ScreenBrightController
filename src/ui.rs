//! Backend-owned ephemeral controls; live_control is the intentional user-gesture write boundary.
use crate::{controller::Outcome, dimming_percent, session::Session};
use serde::Serialize;
use std::collections::BTreeMap;
pub type PopupRect = (i32, i32, u32, u32);
#[derive(Clone, Debug, PartialEq)]
pub struct PopupArea {
    pub work: PopupRect,
    pub scale: f64,
}
/// Reuse actual native geometry; the window owns session-only move/resize memory.
pub fn popup_reopen_bounds(areas: &[PopupArea], current: PopupRect) -> Option<(usize, PopupRect)> {
    let (x, y, width, height) = current;
    let (x, y) = (i64::from(x), i64::from(y));
    let index = areas
        .iter()
        .enumerate()
        .min_by_key(|(_, area)| {
            let (left, top, w, h) = area.work;
            let (left, top) = (i64::from(left), i64::from(top));
            let (right, bottom) = (left + i64::from(w), top + i64::from(h));
            let overlap_w = (right.min(x + i64::from(width)) - left.max(x)).max(0) as u64;
            let overlap_h = (bottom.min(y + i64::from(height)) - top.max(y)).max(0) as u64;
            let (cx, cy) = (2 * x + i64::from(width), 2 * y + i64::from(height));
            let dx = (cx - cx.clamp(2 * left, 2 * right)).unsigned_abs() as u128;
            let dy = (cy - cy.clamp(2 * top, 2 * bottom)).unsigned_abs() as u128;
            (std::cmp::Reverse(overlap_w * overlap_h), dx * dx + dy * dy)
        })?
        .0;
    let area = &areas[index];
    let (left, top, work_w, work_h) = area.work;
    let (min_w, min_h) = popup_min_size(area);
    let width = width.max(min_w).min(work_w);
    let height = height.max(min_h).min(work_h);
    let x = x.clamp(i64::from(left), i64::from(left) + i64::from(work_w - width));
    let y = y.clamp(i64::from(top), i64::from(top) + i64::from(work_h - height));
    Some((index, (x as i32, y as i32, width, height)))
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
/// Minimum readable logical controls, bounded by the actual physical work area.
pub fn popup_min_size(area: &PopupArea) -> (u32, u32) {
    (
        physical(430.0, area.scale).min(area.work.2),
        physical(260.0, area.scale).min(area.work.3),
    )
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
        })
    }
    /// Authoritative recovery state; read-only and independent of request errors.
    pub fn needs_attention(&self) -> bool {
        self.live_blocked || self.recovery.is_some()
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
        if !self.controls.contains_key(id) {
            return Err("unknown control".into());
        }
        self.controls.insert(id.into(), Control { dim, enabled });
        self.revision += 1;
        let result = self.apply();
        if let Err(ref e) = result {
            self.live_blocked = true;
            self.recovery = Some(Recovery::Apply);
            self.generation += 1;
            self.message = format!("Live update stopped: {e}. Restore before retrying.");
        }
        result
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        if let Err(e) = self.session.heartbeat() {
            self.recovery = Some(Recovery::Safety);
            self.message = format!("Safety stop: {e}. Restore before retrying.");
            self.live_blocked = true;
            self.generation += 1;
            self.revision += 1;
            return Err(e);
        }
        Ok(())
    }
    fn apply(&mut self) -> Result<(), String> {
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
            let failed = rows
                .iter()
                .any(|r| !r.api_success || r.readback_matches != Some(true));
            self.outcomes.extend(rows);
            if failed {
                return Err("API rejected, readback failed or driver ignored dimming; attempted targets remain watchdog protected".into());
            }
        }
        self.message="Continuous operation applied. Closing hides to tray; Restore or Quit returns saved originals. Native lease protection active.".into();
        Ok(())
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
                self.recovery = Some(Recovery::Restore);
                self.message = format!("Restore failed: {e}. Keep the app open and retry.");
                Err(e)
            }
        }
    }
}
