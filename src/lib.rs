extern crate byteorder;
extern crate libc;
#[macro_use(defer)]
extern crate scopeguard;

pub mod core;

#[cfg(target_os = "linux")]
pub mod linux;
