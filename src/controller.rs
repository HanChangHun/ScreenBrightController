use crate::Ramp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub original: Option<Ramp>,
    pub error: Option<String>,
}
pub trait Driver {
    fn snapshot(&mut self) -> Result<Vec<Monitor>, String>;
    fn arm_continuous(&mut self, id: &str) -> Result<(), String>;
    fn renew(&mut self, _id: &str) -> Result<(), String> {
        Ok(())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String>;
    fn read(&mut self, id: &str) -> Result<Ramp, String>;
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String>;
    /// Write `linear` as the new saved original; the real backend asks its watchdog.
    fn reset(&mut self, id: &str, linear: &Ramp) -> Result<(), String> {
        self.restore(id, linear)
    }
}
/// Restore error for a display that is not attached; its original stays armed for retry.
pub const DISCONNECTED: &str = "display disconnected, original retained for retry";
/// True when every part of a restore error (`id: error; ...`) is a disconnected display.
pub fn detached_only(error: &str) -> bool {
    !error.is_empty() && error.split("; ").all(|part| part.ends_with(DISCONNECTED))
}
pub fn monitor<'a>(monitors: &'a [Monitor], id: &str) -> Option<&'a Monitor> {
    monitors.iter().find(|m| m.id == id)
}
/// User-initiated recovery from a dimmed saved original (see `looks_dimmed`); never automatic.
pub fn reset_baseline(
    driver: &mut impl Driver,
    monitors: &mut [Monitor],
    id: &str,
) -> Result<(), String> {
    let monitor = monitors
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or("unknown display")?;
    if !monitor.original.as_ref().is_some_and(crate::looks_dimmed) {
        return Err("saved gamma is not dimmed".into());
    }
    let linear = crate::linear_ramp();
    driver.reset(id, &linear)?;
    monitor.original = Some(linear);
    Ok(())
}
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub id: String,
    pub api_success: bool,
    pub readback_matches: Option<bool>,
    pub readback_error: Option<String>,
    /// Readback still equals the last verified ramp: the write was rejected or ignored.
    pub kept_previous: bool,
}
pub struct Controller<D: Driver> {
    pub driver: D,
    pub monitors: Vec<Monitor>,
    pub changed: BTreeSet<String>,
    pub expected: BTreeMap<String, Ramp>,
}
impl<D: Driver> Controller<D> {
    pub fn apply_continuous(
        &mut self,
        ids: &[String],
        percent: u8,
    ) -> Result<Vec<Outcome>, String> {
        // Validate every target before the first arm or write.
        let mut targets: Vec<(&String, Ramp)> = vec![];
        for id in ids {
            if targets.iter().any(|(t, _)| *t == id) {
                return Err("duplicate display".into());
            }
            let original = monitor(&self.monitors, id)
                .and_then(|m| m.original.clone())
                .ok_or("unknown display")?;
            crate::scale(&original, percent)?;
            targets.push((id, original));
        }
        let mut out = vec![];
        for (id, original) in targets {
            if percent == 100 {
                self.restore_target(id)?;
                continue;
            }
            let ramp = crate::scale(&original, percent)?;
            if self.changed.contains(id) {
                self.verify_target(id)?;
                self.driver.renew(id)?;
                if self.expected.get(id) == Some(&ramp) {
                    continue;
                }
            } else {
                self.driver.arm_continuous(id)?;
                self.changed.insert(id.clone());
            }
            let previous = self
                .expected
                .get(id)
                .cloned()
                .unwrap_or_else(|| original.clone());
            // Protect the attempted write even if SET/read fails. Never recapture originals.
            self.expected.insert(id.clone(), ramp.clone());
            let api_success = self.driver.set(id, &ramp)?;
            let readback = self.driver.read(id);
            let kept_previous = readback.as_ref() == Ok(&previous) && previous != ramp;
            if kept_previous {
                // An ignored update left the verified ramp intact: renew it without more SETs.
                self.expected.insert(id.clone(), previous);
            }
            out.push(Outcome {
                id: id.clone(),
                api_success,
                readback_matches: readback.as_ref().ok().map(|r| r == &ramp),
                readback_error: readback.err(),
                kept_previous,
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
        for id in self.changed.iter().cloned().collect::<Vec<_>>() {
            self.verify_target(&id)?;
            self.driver.renew(&id)?;
        }
        Ok(())
    }
    /// Forget displays the watchdog already restored after their lease lapsed (e.g. sleep).
    pub fn release_unarmed(&mut self, armed: &[String]) -> Result<(), String> {
        let lost: Vec<String> = self
            .changed
            .iter()
            .filter(|id| !armed.contains(id))
            .cloned()
            .collect();
        for id in &lost {
            self.changed.remove(id);
            self.expected.remove(id);
        }
        if lost.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{}: lease expired and the watchdog restored the original",
                lost.join(", ")
            ))
        }
    }
    pub fn restore_target(&mut self, id: &str) -> Result<(), String> {
        if !self.changed.contains(id) {
            return Ok(());
        }
        let original = monitor(&self.monitors, id)
            .and_then(|m| m.original.as_ref())
            .ok_or("unknown display")?;
        self.driver.restore(id, original)?;
        self.changed.remove(id);
        self.expected.remove(id);
        Ok(())
    }
    pub fn reset_baseline(&mut self, id: &str) -> Result<(), String> {
        if self.changed.contains(id) {
            return Err("restore before resetting".into());
        }
        reset_baseline(&mut self.driver, &mut self.monitors, id)
    }
    pub fn restore_all(&mut self) -> Result<(), String> {
        let errors: Vec<String> = self
            .changed
            .clone()
            .into_iter()
            .filter_map(|id| self.restore_target(&id).err().map(|e| format!("{id}: {e}")))
            .collect();
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
            changed: BTreeSet::new(),
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
    /// Displays whose SET is ignored, like `ignored_set` for one display.
    pub ignored_ids: Vec<String>,
    /// Displays that behave as unplugged: reads and restores fail.
    pub detached: Vec<String>,
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
    fn arm_continuous(&mut self, id: &str) -> Result<(), String> {
        if self.fail_arm {
            return Err("watchdog unavailable".into());
        }
        self.armed.push(id.into());
        Ok(())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String> {
        self.writes += 1;
        if !self.ignored_set && !self.ignored_ids.iter().any(|i| i == id) {
            self.current.insert(id.into(), ramp.clone());
        }
        Ok(!self.fail_set)
    }
    fn read(&mut self, id: &str) -> Result<Ramp, String> {
        if self.detached.iter().any(|d| d == id) {
            return Err("display unavailable".into());
        }
        self.current
            .get(id)
            .cloned()
            .ok_or("unknown display".into())
    }
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String> {
        if self.detached.iter().any(|d| d == id) {
            return Err(DISCONNECTED.into());
        }
        if self.fail_restore {
            return Err("mock restore failure".into());
        }
        self.writes += 1;
        self.restored.push(id.into());
        self.current.insert(id.into(), original.clone());
        Ok(())
    }
}
