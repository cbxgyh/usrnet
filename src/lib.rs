#![feature(proc_macro)]
extern crate byteorder;
extern crate libc;
extern crate mock_derive;

pub mod core;

#[cfg(target_os = "linux")]
pub mod linux;
