//! Backend-owned ephemeral controls shared by all windows. Editing never calls gamma set.
use crate::{dimming_percent, session::Session};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
/// Coordinates are physical throughout; only the desired logical size/gap is DPI-scaled.
pub fn popup_bounds(
    work: (i32, i32, u32, u32),
    click: (f64, f64),
    scale: f64,
) -> (i32, i32, u32, u32) {
    let (left, top, width, height) = work;
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let w = ((430.0 * scale).round() as u32).min(width);
    let h = ((540.0 * scale).round() as u32).min(height);
    let gap = (12.0 * scale).round() as i32;
    let x = (click.0.round() as i32 - w as i32 - gap).clamp(left, left + width as i32 - w as i32);
    let y = (click.1.round() as i32 - h as i32 - gap).clamp(top, top + height as i32 - h as i32);
    (x, y, w, h)
}
#[derive(Clone, Serialize)]
pub struct Control {
    pub dim: i64,
    pub enabled: bool,
}
pub struct UiSession {
    pub session: Session,
    pub controls: BTreeMap<String, Control>,
    pub consent: bool,
    pub outcomes: Value,
    pub message: String,
}
impl UiSession {
    pub fn new(mut session: Session) -> Result<Self, String> {
        let status = session.status()?;
        let mut controls = BTreeMap::from([(
            "master".into(),
            Control {
                dim: 0,
                enabled: true,
            },
        )]);
        for monitor in status["monitors"].as_array().ok_or("missing monitors")? {
            controls.insert(
                monitor["id"].as_str().ok_or("missing id")?.into(),
                Control {
                    dim: 0,
                    enabled: monitor["supported"] == true,
                },
            );
        }
        Ok(Self {
            session,
            controls,
            consent: false,
            outcomes: json!([]),
            message: "No changes applied.".into(),
        })
    }
    pub fn status(&mut self) -> Result<Value, String> {
        let mut status = self.session.status()?;
        status["controls"] = json!(self.controls);
        status["consent"] = json!(self.consent);
        status["outcomes"] = self.outcomes.clone();
        status["message"] = json!(self.message);
        status["operation"] = json!(if self.session.is_continuous() {
            "continuous"
        } else if !status["armed"].as_array().unwrap().is_empty() {
            "preview"
        } else {
            "idle"
        });
        Ok(status)
    }
    pub fn set_control(&mut self, id: &str, dim: i64, enabled: bool) -> Result<(), String> {
        dimming_percent(dim)?;
        if !self.controls.contains_key(id) {
            return Err("unknown control".into());
        }
        if id == "master" && enabled && dim == 0 {
            self.session.restore()?;
        } else if id != "master" && (!enabled || (dim == 0 && !self.controls["master"].enabled)) {
            self.session.restore_target(id)?;
        }
        let control = self.controls.get_mut(id).ok_or("unknown control")?;
        *control = Control { dim, enabled };
        Ok(())
    }
    pub fn main_close(&mut self) -> Result<(), String> {
        if self.session.is_continuous() {
            Ok(())
        } else {
            self.restore()
        }
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        if let Err(e) = self.session.heartbeat() {
            self.message = format!("Safety stop: {e}. Restore before retrying.");
            self.consent = false;
            return Err(e);
        }
        Ok(())
    }
    pub fn apply(&mut self, mode: &str) -> Result<(), String> {
        if mode == "preview" {
            return self.preview();
        }
        if mode != "continuous" || !self.consent {
            return Err("valid mode and deliberate consent required".into());
        }
        let status = self.session.status()?;
        if !self.session.is_continuous() && !status["armed"].as_array().unwrap().is_empty() {
            return Err("restore preview before continuous apply".into());
        }
        let master = self.controls["master"].clone();
        let mut targets = vec![];
        for monitor in status["monitors"].as_array().ok_or("missing monitors")? {
            let id = monitor["id"].as_str().ok_or("missing id")?;
            let control = &self.controls[id];
            let dim = if master.enabled {
                master.dim
            } else {
                control.dim
            };
            let percent = dimming_percent(dim)?;
            targets.push((
                id.to_string(),
                if control.enabled && monitor["supported"] == true {
                    percent
                } else {
                    100
                },
            ));
        }
        self.consent = false;
        self.outcomes = json!([]);
        let mut out = vec![];
        for (id, percent) in targets {
            if percent == 100 {
                self.session.restore_target(&id)?;
                continue;
            }
            match self.session.continuous(vec![id], percent, true) {
                Ok(mut rows) => out.append(&mut rows),
                Err(e) => {
                    self.outcomes = json!(out);
                    self.message =
                        format!("Apply stopped: {e}. Attempted targets remain watchdog protected.");
                    return Err(e);
                }
            }
        }
        self.outcomes = json!(out);
        self.message="Continuous operation applied. Closing hides to tray; Restore or Quit returns saved originals. Native lease protection active.".into();
        Ok(())
    }
    pub fn preview(&mut self) -> Result<(), String> {
        let status = self.session.status()?;
        if !self.consent
            || !status["armed"]
                .as_array()
                .ok_or("missing armed")?
                .is_empty()
        {
            return Err("explicit consent required; restore before another preview".into());
        }
        let master = &self.controls["master"];
        let mut targets = vec![];
        // Validate the entire request before any arm/write; unsupported/disabled targets are excluded.
        for monitor in status["monitors"].as_array().ok_or("missing monitors")? {
            let id = monitor["id"].as_str().ok_or("missing id")?;
            let control = &self.controls[id];
            let dim = if master.enabled {
                master.dim
            } else {
                control.dim
            };
            let percent = dimming_percent(dim)?;
            if control.enabled && monitor["supported"] == true && dim != 0 {
                targets.push((id.to_string(), percent));
            }
        }
        self.consent = false;
        self.outcomes = json!([]);
        self.message = "No nonzero enabled targets; no gamma writes.".into();
        let mut outcomes = vec![];
        for (id, percent) in targets {
            match self.session.preview(vec![id], percent, true) {
                Ok(mut out) => outcomes.append(&mut out),
                Err(e) => {
                    self.outcomes = json!(outcomes);
                    self.message=format!("Preview error: {e}. Already attempted targets remain watchdog-protected; restore before retry.");
                    return Err(e);
                }
            }
        }
        if !outcomes.is_empty() {
            self.message =
                "15-second preview attempted. API/readback do not verify visible effect.".into();
        }
        self.outcomes = json!(outcomes);
        Ok(())
    }
    pub fn restore(&mut self) -> Result<(), String> {
        self.consent = false;
        match self.session.restore() {
            Ok(()) => {
                for control in self.controls.values_mut() {
                    control.dim = 0;
                }
                self.outcomes = json!([]);
                self.message="Saved original gamma restored for attempted targets only. Visible effect unverified.".into();
                Ok(())
            }
            Err(e) => {
                self.message = format!("Restore failed: {e}. Keep the app open and retry.");
                Err(e)
            }
        }
    }
}
