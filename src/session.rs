use crate::{
    controller::{Controller, Mock, Monitor, Outcome},
    watchdog::Real,
};
use serde_json::{json, Value};
pub enum Session {
    Real(Controller<Real>),
    Demo(Controller<Mock>),
}
impl Session {
    pub fn real() -> Result<Self, String> {
        Ok(Self::Real(Controller::new(Real::new()?)?))
    }
    pub fn demo() -> Result<Self, String> {
        Ok(Self::Demo(Controller::new(Mock::default())?))
    }
    pub fn status(&mut self) -> Result<Value, String> {
        let (mode, monitors, armed, errors) = match self {
            Self::Real(c) => {
                let status = c.driver.status()?;
                ("real", &c.monitors, status.armed, status.restore_errors)
            }
            Self::Demo(c) => (
                "demo",
                &c.monitors,
                c.changed.iter().cloned().collect(),
                vec![],
            ),
        };
        Ok(
            json!({"mode":mode,"monitors":monitors.iter().map(|m|json!({"id":m.id,"name":m.name,"supported":m.original.is_some(),"error":m.error,"dimmed":m.original.as_ref().is_some_and(crate::looks_dimmed)})).collect::<Vec<_>>(),"armed":armed,"restore_errors":errors,"visible_effect_verified":false}),
        )
    }
    /// Report, never hide, displays whose lease lapsed: the slider would show a stale value.
    fn sync_leases(&mut self) -> Result<(), String> {
        if let Self::Real(c) = self {
            let armed = c.driver.status()?.armed;
            c.release_unarmed(&armed)?;
        }
        Ok(())
    }
    pub fn continuous(&mut self, ids: Vec<String>, percent: u8) -> Result<Vec<Outcome>, String> {
        self.sync_leases()?;
        match self {
            Self::Real(c) => c.apply_continuous(&ids, percent),
            Self::Demo(c) => c.apply_continuous(&ids, percent),
        }
    }
    pub fn is_continuous(&self) -> bool {
        match self {
            Self::Real(c) => !c.changed.is_empty(),
            Self::Demo(c) => !c.changed.is_empty(),
        }
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        self.sync_leases()?;
        match self {
            Self::Real(c) => c.heartbeat(),
            Self::Demo(c) => c.heartbeat(),
        }
    }
    pub fn monitors(&self) -> &[Monitor] {
        match self {
            Self::Real(c) => &c.monitors,
            Self::Demo(c) => &c.monitors,
        }
    }
    pub fn reset_baseline(&mut self, id: &str) -> Result<(), String> {
        match self {
            Self::Real(c) => c.reset_baseline(id),
            Self::Demo(c) => c.reset_baseline(id),
        }
    }
    pub fn restore_target(&mut self, id: &str) -> Result<(), String> {
        match self {
            Self::Real(c) => c.restore_target(id),
            Self::Demo(c) => c.restore_target(id),
        }
    }
    pub fn restore(&mut self) -> Result<(), String> {
        match self {
            Self::Real(c) => c.restore_all(),
            Self::Demo(c) => c.restore_all(),
        }
    }
}
