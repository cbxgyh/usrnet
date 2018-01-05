extern crate byteorder;
extern crate libc;

pub mod core;

#[cfg(target_os = "linux")]
pub mod linux;
