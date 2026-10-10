use crate::{
    controller::{Controller, Mock, Monitor, Outcome},
    watchdog::Real,
};
use serde_json::{json, Value};
use std::time::Instant;
pub enum Session {
    Real(Controller<Real>),
    Demo(Controller<Mock>, Option<Instant>),
}
impl Session {
    pub fn real() -> Result<Self, String> {
        Ok(Self::Real(Controller::new(Real::new()?)?))
    }
    pub fn demo() -> Result<Self, String> {
        Ok(Self::Demo(Controller::new(Mock::default())?, None))
    }
    pub fn status(&mut self) -> Result<Value, String> {
        let (mode, monitors, armed, errors) = match self {
            Self::Real(c) => {
                let status = c.driver.status()?;
                ("real", &c.monitors, status.armed, status.restore_errors)
            }
            Self::Demo(c, deadline) => {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    c.restore_all()?;
                    *deadline = None;
                }
                (
                    "demo",
                    &c.monitors,
                    c.changed.keys().cloned().collect(),
                    vec![],
                )
            }
        };
        Ok(
            json!({"mode":mode,"monitors":monitors.iter().map(|m|json!({"id":m.id,"name":m.name,"supported":m.original.is_some(),"error":m.error,"dimmed":m.original.as_ref().is_some_and(crate::looks_dimmed)})).collect::<Vec<_>>(),"armed":armed,"restore_errors":errors,"preview_seconds":15,"visible_effect_verified":false}),
        )
    }
    pub fn preview(
        &mut self,
        ids: Vec<String>,
        percent: u8,
        consent: bool,
    ) -> Result<Vec<Outcome>, String> {
        self.status()?;
        match self {
            Self::Real(c) => c.preview(&ids, percent, 15, consent),
            Self::Demo(c, deadline) => {
                let out = c.preview(&ids, percent, 15, consent)?;
                *deadline = Some(Instant::now() + std::time::Duration::from_secs(15));
                Ok(out)
            }
        }
    }
    /// Report, never hide, displays whose lease lapsed: the slider would show a stale value.
    fn sync_leases(&mut self) -> Result<(), String> {
        if let Self::Real(c) = self {
            let armed = c.driver.status()?.armed;
            c.release_unarmed(&armed)?;
        }
        Ok(())
    }
    pub fn continuous(
        &mut self,
        ids: Vec<String>,
        percent: u8,
        consent: bool,
    ) -> Result<Vec<Outcome>, String> {
        self.sync_leases()?;
        match self {
            Self::Real(c) => c.apply_continuous(&ids, percent, consent),
            Self::Demo(c, _) => c.apply_continuous(&ids, percent, consent),
        }
    }
    pub fn is_continuous(&self) -> bool {
        match self {
            Self::Real(c) => c.changed.values().any(|s| *s == 0),
            Self::Demo(c, _) => c.changed.values().any(|s| *s == 0),
        }
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        self.sync_leases()?;
        match self {
            Self::Real(c) => c.heartbeat(),
            Self::Demo(c, _) => c.heartbeat(),
        }
    }
    pub fn monitors(&self) -> &[Monitor] {
        match self {
            Self::Real(c) => &c.monitors,
            Self::Demo(c, _) => &c.monitors,
        }
    }
    pub fn reset_baseline(&mut self, id: &str) -> Result<(), String> {
        match self {
            Self::Real(c) => c.reset_baseline(id),
            Self::Demo(c, _) => c.reset_baseline(id),
        }
    }
    pub fn restore_target(&mut self, id: &str) -> Result<(), String> {
        match self {
            Self::Real(c) => c.restore_target(id),
            Self::Demo(c, _) => c.restore_target(id),
        }
    }
    pub fn restore(&mut self) -> Result<(), String> {
        match self {
            Self::Real(c) => c.restore_all(),
            Self::Demo(c, deadline) => {
                c.restore_all()?;
                *deadline = None;
                Ok(())
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demo_ui_command_flow_requires_consent_then_restores() {
        let mut s = Session::demo().unwrap();
        assert_eq!(s.status().unwrap()["mode"], "demo");
        assert!(s.preview(vec!["mock-1".into()], 75, false).is_err());
        s.preview(vec!["mock-1".into()], 75, true).unwrap();
        assert_eq!(s.status().unwrap()["armed"], json!(["mock-1"]));
        s.restore().unwrap();
        assert_eq!(s.status().unwrap()["armed"], json!([]));
    }
}
