/*
Logging, done with a different frame of mind.

Rather than presenting a spray of misc string bits.

Let's present a list of auditable events.
*/

use std::io::prelude::*;
use chrono::prelude::{DateTime, Utc};

pub struct Event {
    time: DateTime<Utc>,
    key: String,
    value: String
}

impl Event {
    pub fn new(k: &str, v: &str) -> Event {
        Event {
            time: Utc::now(),
            key: k.to_string(),
            value: v.to_string()
        }
    }
}

#[derive(Copy, Clone)]
pub enum ConcernLevel {
    Debug,
    Info,
    Crisis
}

pub enum Concern {
    Debug(Event),
    Info(Event),
    Crisis(Event)
}

#[derive(Clone)]
pub enum AuditTarget {
    StdErr
}
#[derive(Clone)]
pub struct Audit {
    level: ConcernLevel,
    t: AuditTarget
}

impl Audit {
    pub fn new(c: ConcernLevel)->Audit {
        Audit { level: c, t: AuditTarget::StdErr }
    }

    pub fn tell(&self, c: &Concern) {

        match &self.t {
            Stderr => {
                use audit::Concern::{Debug, Info, Crisis};
                let mut stderr = std::io::stderr();
                let (level, e) = match c {
                    Debug(e) => ("DEBUG", e),
                    Info(e) => ("INFO", e),
                    Crisis(e) => ("CRISIS", e)
                };

                match writeln!(&mut stderr, "{}: {} {} {}", e.time, level, e.key, e.value) {
                    Err(e) => panic!("writing to stderr failed, invariant failed, crashing"),
                    Ok(_) => ()
                }
            }
        }
    }
}
