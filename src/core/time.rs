//! Abstractions for providing the current time.

use std::fmt::Debug;
use std::time::Instant;

/// An environment that provides the current time.
pub trait Env: Clone + Debug {
    /// Returns an instance corresponding to "now".
    fn now_instant(&self) -> Instant;
}

/// An environment that provides system based time.
#[derive(Clone, Debug)]
pub struct SystemEnv;

impl SystemEnv {
    pub fn new() -> SystemEnv {
        SystemEnv {}
    }
}

impl Env for SystemEnv {
    fn now_instant(&self) -> Instant {
        Instant::now()
    }
}

/// An environment that provides a configurable time.
#[derive(Clone, Debug)]
pub struct MockEnv {
    pub now: Instant,
}

impl MockEnv {
    pub fn new() -> MockEnv {
        MockEnv {
            now: Instant::now(),
        }
    }
}

impl Env for MockEnv {
    fn now_instant(&self) -> Instant {
        self.now
    }
}
