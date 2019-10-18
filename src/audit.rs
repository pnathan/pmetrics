/*
Logging, done with a different frame of mind.

Rather than presenting a spray of misc string bits.

Let's present a list of auditable events.
 */

extern crate either;
extern crate itertools;

use std::io::prelude::*;
use chrono::prelude::{DateTime, Utc};
use audit::itertools::Itertools;
use audit::either::{Either, Right, Left};

pub struct Event {
    time: DateTime<Utc>,
    // Left: KV - Right - Vec of KVs
    data: Either<(String, String), Vec<(String, String)>>
}

impl Event {
    pub fn new(k: &str, v: &str) -> Event {
        Event {
            time: Utc::now(),
            data: Left((k.to_string(),v.to_string()))
        }
    }
    pub fn newvec(pairs: &[&str]) -> Event {
        let mut kvs: Vec<(String, String)> = Vec::new();
        for chunk in &pairs.into_iter().chunks(2) {
            let pair: Vec<_> = chunk.collect();
            if pair.len() == 2 {
                kvs.push((pair[0].to_string(), pair[1].to_string()));
            }
        }

        Event {
            time: Utc::now(),
            data: Right(kvs)
        }

    }
}

pub fn event(k: &str, v: &str) -> Event {
    Event::new(k,v)
}

pub fn eventw(kvs:  &[&str]) -> Event {
    Event::newvec(kvs)
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

#[derive(Copy, Clone)]
pub enum AuditTarget {
    Stderr(),
    Noop()
}
#[derive(Copy, Clone)]
pub struct Audit {
    level: ConcernLevel,
    t: AuditTarget
}

impl Audit {
    pub fn new(c: ConcernLevel) -> Audit {
        Audit { level: c, t: AuditTarget::Stderr() }
    }

    pub fn debug(&self, event: Event) {
        self.tell(&Concern::Debug(event));
    }
    pub fn info(&self, event: Event) {
        self.tell(&Concern::Info(event));
    }
    pub fn crisis(&self, event: Event) {
        self.tell(&Concern::Crisis(event));
    }
    pub fn tell(&self, c: &Concern) {

        match &self.t {
            AuditTarget::Stderr() => {
                let mut stderr = std::io::stderr();
                let (level, e) = match c {
                    Concern::Debug(e) => ("DEBUG", e),
                    Concern::Info(e) => ("INFO", e),
                    Concern::Crisis(e) => ("CRISIS", e)
                };

                match writeln!(&mut stderr, "{}: {}: {:?}", e.time, level, e.data) {
                    Err(e) => panic!("writing to stderr failed, invariant failed, crashing: {}", e),
                    Ok(_) => ()
                }
            }
            AuditTarget::Noop() => {}
        }
    }
}
