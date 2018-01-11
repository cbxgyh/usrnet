use std;

use libc;

use core::link::{
    Error,
    Link,
    Result,
};
use linux::libc as _libc;

/// [TAP interface](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
/// for sending and receiving raw ethernet frames.
pub struct Tap {
    fd: libc::c_int,
    ifreq: _libc::c_ifreq,
}

impl Tap {
    /// Creates or binds to an existing TAP interface with the specified name.
    ///
    /// # Panics
    ///
    /// Causes a panic if [tun_alloc(...)](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
    /// runs into an error.
    pub fn new(ifr_name: &str) -> Tap {
        unsafe {
            let fd = libc::open(
                "/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR,
            );

            if fd == -1 {
                panic!("Opening TAP: {}.", std::io::Error::last_os_error());
            }

            let ifreq = _libc::c_ifreq::with_name(ifr_name);

            let mut _ifreq = ifreq.clone();
            _ifreq.ifr_ifru.ifr_flags = _libc::IFF_TAP | _libc::IFF_NO_PI;
            if libc::ioctl(fd, _libc::TUNSETIFF, &mut _ifreq as *mut _libc::c_ifreq) == -1 {
                panic!("TUNSETIFF TAP: {}.", std::io::Error::last_os_error());
            }

            Tap { fd, ifreq }
        }
    }
}

impl Link for Tap {
    fn send(&mut self, buf: &[u8]) -> Result<()> {
        unsafe {
            let ptr = buf.as_ptr() as *const libc::c_void;
            if libc::write(self.fd, ptr, buf.len()) == -1 {
                return Err(Error::IO(std::io::Error::last_os_error()));
            }
            Ok(())
        }
    }

    fn recv(&mut self, _: &mut [u8]) -> Result<usize> {
        unimplemented!();
    }

    fn get_max_transmission_unit(&self) -> Result<usize> {
        unsafe {
            let fd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);

            if fd == -1 {
                return Err(Error::IO(std::io::Error::last_os_error()));
            }

            let mut ifreq = self.ifreq.clone();

            if libc::ioctl(fd, _libc::SIOCGIFMTU, &mut ifreq as *mut _libc::c_ifreq) == -1 {
                libc::close(fd);
                return Err(Error::IO(std::io::Error::last_os_error()));
            }

            libc::close(fd);
            Ok(ifreq.ifr_ifru.ifr_mtu as usize)
        }
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
