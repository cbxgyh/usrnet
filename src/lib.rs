#![feature(proc_macro)]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
extern crate byteorder;
extern crate libc;
#[macro_use]
extern crate log;

pub mod core;

#[cfg(target_os = "linux")]
pub mod linux;
