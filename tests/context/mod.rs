use std::process::{
    ExitStatus,
    Output as StdOutput,
};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use usrnet::core::service::Interface;
use usrnet::core::socket::SocketSet;
use usrnet::examples::*;

lazy_static! {
    static ref TEST: Mutex<()> = Mutex::new(());
}

#[derive(Debug)]
pub struct Output {
    pub stderr: String,
    pub stdout: String,
    pub status: ExitStatus,
}

impl From<StdOutput> for Output {
    fn from(output: StdOutput) -> Output {
        Output {
            stderr: String::from_utf8(output.stderr).unwrap(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            status: output.status,
        }
    }
}

/// Runs a function f in an exclusive context. This is important so that tests
/// run independently and do not share the TAP interface.
pub fn run<F, R>(f: F) -> R
where
    F: FnOnce(&mut Interface, &mut SocketSet) -> R,
{
    // Assertion failures cause panics and poison the mutex.
    let _guard = match TEST.lock() {
        Ok(guard) => guard,
        Err(err) => err.into_inner(),
    };

    // Wait a second or so to free the TAP after the previous test shuts down.
    thread::sleep(Duration::from_secs(1));

    f(&mut env::default_interface(), &mut env::socket_set())
}
