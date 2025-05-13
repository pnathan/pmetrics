/*
Logging, done with a different frame of mind.

Rather than presenting a spray of misc string bits.

Let's present a list of auditable events.
 */

extern crate either;
extern crate itertools;

use std::io::prelude::*;
//use std::fmt::Write;

use chrono::prelude::{DateTime, Utc};
use itertools::Itertools;

#[derive(PartialEq, Debug)]
pub struct Event {
    time: DateTime<Utc>,
    // Left: KV - Right - Vec of KVs
    // TODO: collapse this. Left should be a vec of length 1.
    data: Vec<(String, String)>,
}

impl Event {
    // new event with 1 kv pair
    pub fn new(k: &str, v: &str) -> Event {
        Event {
            time: Utc::now(),
            data: vec![(k.to_string(), v.to_string())],
        }
    }
    // new event with multiple kv pairs.
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
            data: kvs,
        }
    }

    pub fn pretty(&self) -> String {
        let mut res = String::new();
        for pair in &self.data {
            res.push_str(&format!("{}={}  ", pair.0, pair.1));
        }
        return res;
    }
}

pub fn event(k: &str, v: &str) -> Event {
    Event::new(k, v)
}

pub fn eventw(kvs: &[&str]) -> Event {
    Event::newvec(kvs)
}

#[derive(Copy, Clone, Debug)]
pub enum ConcernLevel {
    Debug,
    Info,
    Crisis,
}

pub enum Concern {
    Debug(Event),
    Info(Event),
    Crisis(Event),
}

#[derive(Copy, Clone)]
pub enum AuditTarget {
    Stderr(),
    Noop(),
}
#[derive(Copy, Clone)]
pub struct Audit {
    pub level: ConcernLevel,
    pub t: AuditTarget,
}

impl Audit {
    pub fn new(c: ConcernLevel) -> Audit {
        Audit {
            level: c,
            t: AuditTarget::Stderr(),
        }
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
                match &self.level {
                    // pass everything through
                    ConcernLevel::Debug => {}

                    // short-circuit on Debug
                    ConcernLevel::Info => match c {
                        Concern::Debug(_) => return,
                        _ => {}
                    },

                    ConcernLevel::Crisis => match c {
                        // Only keep going on Crisis.
                        Concern::Crisis(_) => {}
                        _ => return,
                    },
                }

                let (level, e) = match c {
                    Concern::Debug(e) => ("DEBUG", e),
                    Concern::Info(e) => ("INFO", e),
                    Concern::Crisis(e) => ("CRISIS", e),
                };

                let mut stderr = std::io::stderr();
                match writeln!(&mut stderr, "{}: {}: {}", e.time, level, e.pretty()) {
                    Err(e) => panic!(
                        "writing to stderr failed, invariant failed, crashing: {}",
                        e
                    ),
                    Ok(_) => (),
                }
            }
            AuditTarget::Noop() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newevent() {
        let gotten = event("a", "b");
        let expected = Event {
            time: gotten.time,
            data: vec![("a".to_string(), "b".to_string())],
        };
        assert_eq!(expected, gotten);
    }

    #[test]
    fn test_newevent_w() {
        let gotten = eventw(&["a", "b", "c", "d"]);
        let expected = Event {
            time: gotten.time,
            data: vec![
                ("a".to_string(), "b".to_string()),
                ("c".to_string(), "d".to_string()),
            ],
        };
        assert_eq!(expected, gotten);
    }

    #[test]
    fn test_target() {
        let a = Audit {
            level: ConcernLevel::Crisis,
            t: AuditTarget::Noop(),
        };
        a.tell(&Concern::Debug(event("a", "b")));
    }
}
