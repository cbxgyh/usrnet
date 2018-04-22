use env_logger;

use std::process::{
    ExitStatus,
    Output as StdOutput,
};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use rand;

use usrnet::core::service::Interface;
use usrnet::core::socket::{
    SocketEnv,
    SocketSet,
};
use usrnet::core::time::SystemEnv;
use usrnet::examples::*;

lazy_static! {
    static ref TEST: Mutex<()> = {
        env_logger::init();
        Mutex::new(())
    };

    static ref PORT: Mutex<u16> = {
        Mutex::new(rand::random::<u16>())
    };

    pub static ref ONE_SEC: Duration = {
        Duration::from_secs(1)
    };
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

pub struct Context {
    pub interface: Interface,
    pub socket_env: SocketEnv<SystemEnv>,
    pub socket_set: SocketSet,
}

impl Default for Context {
    fn default() -> Context {
        let interface = env::default_interface();
        let socket_env = SocketEnv::new(&interface, SystemEnv::new());
        let socket_set = SocketSet::new(32);
        Context {
            interface,
            socket_env,
            socket_set,
        }
    }
}

/// Returns a random-ish UDP/TCP port.
///
/// Guarantees 65,536 unique ports before repeating.
#[allow(dead_code)]
pub fn rand_port() -> u16 {
    let mut port = PORT.lock().unwrap();
    *port += 1;
    *port
}

/// Runs a function f in an exclusive context. This is important so that tests
/// run independently and do not share the TAP interface.
#[allow(dead_code)]
pub fn run<F, R>(f: F) -> R
where
    F: FnOnce(&mut Context) -> R,
{
    let _guard = TEST.lock().unwrap();

    // Wait a second or so for the TAP to shutdown before starting the next test.
    thread::sleep(*ONE_SEC);

    f(&mut Context::default())
}
