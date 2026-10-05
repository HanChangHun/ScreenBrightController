use crate::Ramp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub original: Option<Ramp>,
    pub error: Option<String>,
}
pub trait Driver {
    fn snapshot(&mut self) -> Result<Vec<Monitor>, String>;
    fn arm(&mut self, id: &str, seconds: u64) -> Result<(), String>;
    fn arm_continuous(&mut self, id: &str) -> Result<(), String> {
        self.arm(id, 10)
    }
    fn renew(&mut self, _id: &str) -> Result<(), String> {
        Ok(())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String>;
    fn read(&mut self, id: &str) -> Result<Ramp, String>;
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String>;
}
#[derive(Debug, Serialize)]
pub struct Outcome {
    pub id: String,
    pub api_success: bool,
    pub readback_matches: Option<bool>,
    pub readback_error: Option<String>,
    pub visible_effect_verified: bool,
}
pub struct Controller<D: Driver> {
    pub driver: D,
    pub monitors: Vec<Monitor>,
    pub changed: BTreeMap<String, u64>,
    pub expected: BTreeMap<String, Ramp>,
}
impl<D: Driver> Controller<D> {
    pub fn preview(
        &mut self,
        ids: &[String],
        percent: u8,
        seconds: u64,
        consent: bool,
    ) -> Result<Vec<Outcome>, String> {
        if !consent || !(1..=30).contains(&seconds) || ids.is_empty() {
            return Err("explicit consent and 1..30 second preview required".into());
        }
        let mut unique = std::collections::BTreeSet::new();
        for id in ids {
            if !unique.insert(id) || self.changed.contains_key(id) {
                return Err("restore before starting another preview".into());
            }
            let ramp = self
                .monitors
                .iter()
                .find(|m| &m.id == id)
                .and_then(|m| m.original.as_ref())
                .ok_or("unknown or unsupported display")?;
            crate::scale(ramp, percent)?;
        }
        let mut results = vec![];
        for id in ids {
            let original = self
                .monitors
                .iter()
                .find(|m| &m.id == id)
                .and_then(|m| m.original.as_ref())
                .ok_or("unknown or unsupported display")?;
            let ramp = crate::scale(original, percent)?;
            if &ramp == original {
                continue;
            }
            self.driver.arm(id, seconds)?;
            self.changed.insert(id.clone(), seconds);
            let api_success = self.driver.set(id, &ramp)?;
            let readback = self.driver.read(id);
            results.push(Outcome {
                id: id.clone(),
                api_success,
                readback_matches: readback.as_ref().ok().map(|r| r == &ramp),
                readback_error: readback.err(),
                visible_effect_verified: false,
            });
        }
        Ok(results)
    }
    pub fn apply_continuous(
        &mut self,
        ids: &[String],
        percent: u8,
        consent: bool,
    ) -> Result<Vec<Outcome>, String> {
        if !consent || ids.is_empty() {
            return Err("explicit continuous consent required".into());
        }
        let mut unique = std::collections::BTreeSet::new();
        for id in ids {
            if !unique.insert(id) {
                return Err("duplicate display".into());
            }
            let original = self
                .monitors
                .iter()
                .find(|m| &m.id == id)
                .and_then(|m| m.original.as_ref())
                .ok_or("unknown display")?;
            crate::scale(original, percent)?;
            if self.changed.get(id).is_some_and(|s| *s != 0) {
                return Err("restore preview before continuous apply".into());
            }
        }
        let mut out = vec![];
        for id in ids {
            let original = self
                .monitors
                .iter()
                .find(|m| &m.id == id)
                .and_then(|m| m.original.as_ref())
                .unwrap()
                .clone();
            if percent == 100 {
                self.restore_target(id)?;
                continue;
            }
            let ramp = crate::scale(&original, percent)?;
            if self.changed.contains_key(id) {
                self.verify_target(id)?;
                self.driver.renew(id)?;
            } else {
                self.driver.arm_continuous(id)?;
                self.changed.insert(id.clone(), 0);
            }
            self.expected.insert(id.clone(), ramp.clone());
            let api_success = self.driver.set(id, &ramp)?;
            let readback = self.driver.read(id);
            out.push(Outcome {
                id: id.clone(),
                api_success,
                readback_matches: readback.as_ref().ok().map(|r| r == &ramp),
                readback_error: readback.err(),
                visible_effect_verified: false,
            });
        }
        Ok(out)
    }
    pub fn verify_target(&mut self, id: &str) -> Result<(), String> {
        if let Some(expected) = self.expected.get(id) {
            if self.driver.read(id).as_ref() != Ok(expected) {
                self.restore_target(id)?;
                return Err(format!(
                    "{id}: external color change or readback mismatch; restored and disarmed"
                ));
            }
        }
        Ok(())
    }
    pub fn heartbeat(&mut self) -> Result<(), String> {
        for id in self
            .changed
            .iter()
            .filter(|(_, s)| **s == 0)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
        {
            self.verify_target(&id)?;
            self.driver.renew(&id)?;
        }
        Ok(())
    }
    pub fn restore_target(&mut self, id: &str) -> Result<(), String> {
        if !self.changed.contains_key(id) {
            return Ok(());
        }
        let original = self
            .monitors
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.original.as_ref())
            .ok_or("unknown display")?;
        self.driver.restore(id, original)?;
        self.changed.remove(id);
        self.expected.remove(id);
        Ok(())
    }
    pub fn restore_all(&mut self) -> Result<(), String> {
        let mut errors = vec![];
        for id in self.changed.keys().cloned().collect::<Vec<_>>() {
            let original = self
                .monitors
                .iter()
                .find(|m| m.id == id)
                .and_then(|m| m.original.as_ref())
                .unwrap();
            match self.driver.restore(&id, original) {
                Ok(()) => {
                    self.changed.remove(&id);
                    self.expected.remove(&id);
                }
                Err(e) => errors.push(format!("{id}: {e}")),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
    pub fn new(mut driver: D) -> Result<Self, String> {
        let monitors = driver.snapshot()?;
        Ok(Self {
            driver,
            monitors,
            changed: BTreeMap::new(),
            expected: BTreeMap::new(),
        })
    }
}
#[derive(Default)]
pub struct Mock {
    pub writes: usize,
    pub armed: Vec<String>,
    pub restored: Vec<String>,
    pub fail_arm: bool,
    pub fail_restore: bool,
    pub fail_set: bool,
    pub ignored_set: bool,
    pub current: BTreeMap<String, Ramp>,
}
impl Driver for Mock {
    fn snapshot(&mut self) -> Result<Vec<Monitor>, String> {
        let mut out = vec![];
        for (id, value) in [("mock-1", 40000), ("mock-2", 50000)] {
            let original = vec![vec![value; 256]; 3];
            self.current.entry(id.into()).or_insert(original.clone());
            out.push(Monitor {
                id: id.into(),
                name: id.into(),
                original: Some(original),
                error: None,
            });
        }
        Ok(out)
    }
    fn arm(&mut self, id: &str, _seconds: u64) -> Result<(), String> {
        if self.fail_arm {
            return Err("watchdog unavailable".into());
        }
        self.armed.push(id.into());
        Ok(())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String> {
        self.writes += 1;
        if !self.ignored_set {
            self.current.insert(id.into(), ramp.clone());
        }
        Ok(!self.fail_set)
    }
    fn read(&mut self, id: &str) -> Result<Ramp, String> {
        self.current
            .get(id)
            .cloned()
            .ok_or("unknown display".into())
    }
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String> {
        if self.fail_restore {
            return Err("mock restore failure".into());
        }
        self.writes += 1;
        self.restored.push(id.into());
        self.current.insert(id.into(), original.clone());
        Ok(())
    }
}
