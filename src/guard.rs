use crate::controller::{monitor, Driver, Monitor, DISCONNECTED};
use std::collections::BTreeMap;
pub struct Guard<D: Driver> {
    pub driver: D,
    pub monitors: Vec<Monitor>,
    pub armed: BTreeMap<String, u64>,
    pub continuous: std::collections::BTreeSet<String>,
    /// Latest restore failure per display; cleared when its restore succeeds.
    pub errors: BTreeMap<String, String>,
}
impl<D: Driver> Guard<D> {
    pub fn new(mut driver: D) -> Result<Self, String> {
        let monitors = driver.snapshot()?;
        Ok(Self {
            driver,
            monitors,
            armed: BTreeMap::new(),
            continuous: Default::default(),
            errors: BTreeMap::new(),
        })
    }
    pub fn arm_continuous(&mut self, id: &str, lease: u64, now: u64) -> Result<(), String> {
        if lease != 10 {
            return Err("continuous lease must be 10 seconds".into());
        }
        if self.armed.contains_key(id) {
            return Err("already armed".into());
        }
        let original = monitor(&self.monitors, id)
            .and_then(|m| m.original.as_ref())
            .ok_or("unknown display or unreadable original")?;
        crate::scale(original, 100)?;
        self.armed
            .insert(id.into(), now.saturating_add(lease * 1000));
        self.continuous.insert(id.into());
        Ok(())
    }
    pub fn renew(&mut self, id: &str, lease: u64, now: u64) -> Result<(), String> {
        if lease != 10
            || !self.continuous.contains(id)
            || !self.armed.get(id).is_some_and(|d| *d > now)
        {
            return Err("expired, unknown, or non-continuous lease".into());
        }
        self.armed
            .insert(id.into(), now.saturating_add(lease * 1000));
        Ok(())
    }
    pub fn tick(&mut self, now: u64, parent_alive: bool) {
        let ids = self
            .armed
            .iter()
            // Parent death restores at once; a failed restore retries on its own schedule.
            .filter(|(id, deadline)| {
                **deadline <= now || (!parent_alive && !self.errors.contains_key(*id))
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            // Expiry/death revokes renewal even when restoration must be retried.
            self.continuous.remove(&id);
            if let Err(e) = self.restore(&id) {
                self.armed.insert(id.clone(), now.saturating_add(1000));
                self.errors.insert(id, e);
            }
        }
    }
    /// Every remaining target failed only because its display is unplugged.
    pub fn waiting_for_display(&self) -> bool {
        !self.armed.is_empty()
            && self
                .armed
                .keys()
                .all(|id| self.errors.get(id).is_some_and(|e| e == DISCONNECTED))
    }
    pub fn reset(&mut self, id: &str) -> Result<(), String> {
        if self.armed.contains_key(id) {
            return Err("restore before resetting".into());
        }
        crate::controller::reset_baseline(&mut self.driver, &mut self.monitors, id)
    }
    pub fn restore(&mut self, id: &str) -> Result<(), String> {
        let monitor = monitor(&self.monitors, id).ok_or("unknown display")?;
        if !self.armed.contains_key(id) {
            return Ok(());
        }
        let original = monitor.original.as_ref().ok_or("missing owned original")?;
        self.driver.restore(id, original)?;
        self.armed.remove(id);
        self.continuous.remove(id);
        self.errors.remove(id);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Mock;
    #[test]
    fn parent_death_restores_immediately_and_unarmed_never_writes() {
        let mut g = Guard::new(Mock::default()).unwrap();
        g.tick(1, false);
        assert_eq!(g.driver.writes, 0);
        g.arm_continuous("mock-2", 10, 1).unwrap();
        g.driver.set("mock-2", &vec![vec![100; 256]; 3]).unwrap();
        g.tick(2, false);
        assert_eq!(g.driver.current["mock-2"][0][0], 50000);
        assert!(g.armed.is_empty());
    }
    #[test]
    fn timer_restores_owned_original_only_after_deadline() {
        let mut g = Guard::new(Mock::default()).unwrap();
        g.arm_continuous("mock-1", 10, 100).unwrap();
        g.driver.set("mock-1", &vec![vec![100; 256]; 3]).unwrap();
        g.tick(10099, true);
        assert!(g.driver.restored.is_empty());
        g.tick(10100, true);
        assert_eq!(g.driver.current["mock-1"][0][0], 40000);
        assert!(g.armed.is_empty());
    }
    #[test]
    fn retried_restore_keeps_one_error_and_clears_it_on_success() {
        let mut g = Guard::new(Mock {
            fail_restore: true,
            ..Default::default()
        })
        .unwrap();
        g.arm_continuous("mock-1", 10, 0).unwrap();
        for now in (10000..20000).step_by(1000) {
            g.tick(now, true);
        }
        assert_eq!(g.errors.len(), 1);
        g.driver.fail_restore = false;
        g.tick(20000, true);
        assert!(g.armed.is_empty());
        assert!(g.errors.is_empty());
    }
}
