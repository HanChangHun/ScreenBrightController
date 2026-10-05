use crate::controller::{Driver, Monitor};
use std::collections::BTreeMap;
pub struct Guard<D: Driver> {
    pub driver: D,
    pub monitors: Vec<Monitor>,
    pub armed: BTreeMap<String, u64>,
    pub continuous: std::collections::BTreeSet<String>,
    pub errors: Vec<String>,
}
impl<D: Driver> Guard<D> {
    pub fn new(mut driver: D) -> Result<Self, String> {
        let monitors = driver.snapshot()?;
        Ok(Self {
            driver,
            monitors,
            armed: BTreeMap::new(),
            continuous: Default::default(),
            errors: vec![],
        })
    }
    pub fn arm(&mut self, id: &str, seconds: u64, now: u64) -> Result<(), String> {
        if !(1..=30).contains(&seconds) || self.armed.contains_key(id) {
            return Err("invalid duration or already armed".into());
        }
        let original = self
            .monitors
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.original.as_ref())
            .ok_or("unknown display or unreadable original")?;
        crate::scale(original, 100)?;
        self.armed
            .insert(id.into(), now.saturating_add(seconds * 1000));
        Ok(())
    }
    pub fn arm_continuous(&mut self, id: &str, lease: u64, now: u64) -> Result<(), String> {
        if lease != 10 {
            return Err("continuous lease must be 10 seconds".into());
        }
        self.arm(id, lease, now)?;
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
            .filter(|(_, deadline)| !parent_alive || **deadline <= now)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            // Expiry/death revokes renewal even when restoration must be retried.
            self.continuous.remove(&id);
            if let Err(e) = self.restore(&id) {
                self.errors.push(format!("{id}: {e}"));
                self.armed.insert(id, now.saturating_add(1000));
            }
        }
    }
    pub fn restore(&mut self, id: &str) -> Result<(), String> {
        if !self.monitors.iter().any(|m| m.id == id) {
            return Err("unknown display".into());
        }
        if !self.armed.contains_key(id) {
            return Ok(());
        }
        let original = self
            .monitors
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.original.as_ref())
            .ok_or("missing owned original")?;
        self.driver.restore(id, original)?;
        self.armed.remove(id);
        self.continuous.remove(id);
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
        g.arm("mock-2", 30, 1).unwrap();
        g.driver.set("mock-2", &vec![vec![100; 256]; 3]).unwrap();
        g.tick(2, false);
        assert_eq!(g.driver.current["mock-2"][0][0], 50000);
        assert!(g.armed.is_empty());
    }
    #[test]
    fn timer_restores_owned_original_only_after_deadline() {
        let mut g = Guard::new(Mock::default()).unwrap();
        g.arm("mock-1", 2, 100).unwrap();
        g.driver.set("mock-1", &vec![vec![100; 256]; 3]).unwrap();
        g.tick(2099, true);
        assert!(g.driver.restored.is_empty());
        g.tick(2100, true);
        assert_eq!(g.driver.current["mock-1"][0][0], 40000);
        assert!(g.armed.is_empty());
    }
}
