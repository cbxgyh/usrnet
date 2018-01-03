use std;

use libc;

use core::link::{Link, Result};
use core::storage::{Buffer, BufferMut};

// TODO: Migrate to https://github.com/rust-lang/libc/blob/master/src/unix/notbsd/linux/mod.rs
const IFF_TAP: libc::c_short = 0x0002;

const IFF_NO_PI: libc::c_short = 0x1000;

const TUNSETIFF: libc::c_ulong = 0x400454CA;

// http://man7.org/linux/man-pages/man7/netdevice.7.html
#[repr(C)]
#[derive(Debug)]
struct ifreq {
    ifr_name: [libc::c_char; libc::IF_NAMESIZE],
    ifr_flags: libc::c_short,
}

impl ifreq {
    pub fn with_name(ifr_name: &str) -> ifreq {
        assert!(ifr_name.len() <= libc::IF_NAMESIZE);

        let mut ifreq = ifreq {
            ifr_name: [0; libc::IF_NAMESIZE],
            ifr_flags: 0,
        };

        for (i, c) in ifr_name.as_bytes().iter().enumerate() {
            ifreq.ifr_name[i] = *c as libc::c_char;
        }

        ifreq
    }
}

/// [TAP interface](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
/// for sending and receiving raw ethernet frames.
pub struct Tap {
    fd: libc::c_int,
}

impl Tap {
    /// Creates or binds to an existing TAP interface with the specified name.
    ///
    /// # Panics
    ///
    /// Causes a panic if any of the operations in [tun_alloc]
    /// (https://www.kernel.org/doc/Documentation/networking/tuntap.txt) error.
    pub fn new(ifr_name: &str) -> Tap {
        let fd = unsafe {
            libc::open(
                "/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR,
            )
        };

        if fd == -1 {
            panic!("Opening TAP: {}.", std::io::Error::last_os_error());
        }

        let mut ifreq = ifreq::with_name(ifr_name);

        ifreq.ifr_flags = IFF_TAP | IFF_NO_PI;

        if unsafe { libc::ioctl(fd, TUNSETIFF, &mut ifreq as *mut ifreq) } == -1 {
            unsafe {
                libc::close(fd);
            }
            panic!("TUNSETIFF TAP: {}.", std::io::Error::last_os_error());
        }

        Tap { fd }
    }
}

impl Link for Tap {
    fn recv(_: &mut BufferMut) -> Result<()> {
        Ok(())
    }

    fn send(_: &Buffer) -> Result<()> {
        Ok(())
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
