use std::io::Error as IOError;

use libc;

use {
    Error,
    Result,
};
use core::dev::Device;
use linux::libc as _libc;

/// [TAP interface](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
/// for sending and receiving raw ethernet frames.
pub struct Tap {
    tapfd: libc::c_int,
    max_transmission_unit: usize,
}

impl Tap {
    /// Creates or binds to an existing TAP interface with the specified IP and
    /// ethernet address.
    ///
    /// # Panics
    ///
    /// Causes a panic if [tun_alloc(...)](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
    /// runs into an error.
    pub fn new(ifr_name: &str) -> Tap {
        unsafe {
            let ifreq = _libc::c_ifreq::with_name(ifr_name);

            // Create the TAP...
            let tapfd = libc::open(
                "/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK,
            );

            if tapfd < 0 {
                panic!("Opening TAP: {}.", IOError::last_os_error());
            }

            let mut _ifreq = ifreq.clone();
            _ifreq.ifr_ifru.ifr_flags = _libc::IFF_TAP | _libc::IFF_NO_PI;
            if libc::ioctl(tapfd, _libc::TUNSETIFF, &mut _ifreq as *mut _libc::c_ifreq) == -1 {
                panic!("TUNSETIFF TAP: {}.", IOError::last_os_error());
            }

            // Query the MTU...
            let sockfd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);

            if sockfd == -1 {
                panic!("Opening socket: {}.", IOError::last_os_error());
            }

            let mut _ifreq = ifreq.clone();

            if libc::ioctl(
                sockfd,
                _libc::SIOCGIFMTU,
                &mut _ifreq as *mut _libc::c_ifreq,
            ) == -1
            {
                panic!("IOCTL socket: {}.", IOError::last_os_error());
            }

            libc::close(sockfd);

            let max_transmission_unit = _ifreq.ifr_ifru.ifr_mtu as usize;

            // Now we're done!
            Tap {
                tapfd,
                max_transmission_unit,
            }
        }
    }
}

impl Device for Tap {
    fn send(&mut self, buffer: &[u8]) -> Result<()> {
        unsafe {
            let wrote = libc::write(
                self.tapfd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
            );

            if wrote < 0 && _libc::errno() == libc::EAGAIN {
                return Err(Error::Exhausted);
            } else if wrote < 0 {
                Err(Error::IO(IOError::last_os_error()))
            } else {
                Ok(())
            }
        }
    }

    fn recv(&mut self, buffer: &mut [u8]) -> Result<usize> {
        unsafe {
            let read = libc::read(
                self.tapfd,
                buffer.as_ptr() as *mut libc::c_void,
                buffer.len(),
            );

            if read < 0 && _libc::errno() == libc::EAGAIN {
                return Err(Error::Exhausted);
            } else if read < 0 {
                Err(Error::IO(IOError::last_os_error()))
            } else {
                Ok(read as usize)
            }
        }
    }

    fn max_transmission_unit(&self) -> usize {
        self.max_transmission_unit
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.tapfd);
        }
    }
}
