use std;

/// An environment that provides the current time.
pub trait Env {
    /// Returns an instance corresponding to "now".
    fn now_instant(&self) -> std::time::Instant;
}

/// An environment that provides system based time.
pub struct SystemEnv;

impl SystemEnv {
    pub fn new() -> SystemEnv {
        SystemEnv {}
    }
}

impl Env for SystemEnv {
    fn now_instant(&self) -> std::time::Instant {
        std::time::Instant::now()
    }
}

/// An environment that provides a configurable time.
pub struct MockEnv {
    pub now: std::time::Instant,
}

impl MockEnv {
    pub fn new() -> MockEnv {
        MockEnv {
            now: std::time::Instant::now(),
        }
    }
}

impl Env for MockEnv {
    fn now_instant(&self) -> std::time::Instant {
        self.now
    }
}
