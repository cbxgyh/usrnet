//! Abstractions for providing the current time.

use std::time::Instant;

/// An environment that provides the current time.
pub trait Env {
    /// Returns an instance corresponding to "now".
    fn now_instant(&self) -> Instant;
}

/// An environment that provides system based time.
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
