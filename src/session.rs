use crate::{
    controller::{Controller, Mock, Monitor, Outcome},
    watchdog::Real,
};
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
    /// Watchdog restore failures; asking also proves the watchdog still answers.
    pub fn restore_errors(&mut self) -> Result<Vec<String>, String> {
        match self {
            Self::Real(c) => Ok(c.driver.status()?.restore_errors),
            Self::Demo(_) => Ok(vec![]),
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
    pub fn continuous(&mut self, ids: Vec<String>, percent: u8) -> Result<Vec<Outcome>, String> {
        self.sync_leases()?;
        match self {
            Self::Real(c) => c.apply_continuous(&ids, percent),
            Self::Demo(c) => c.apply_continuous(&ids, percent),
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
