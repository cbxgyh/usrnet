use std;

use libc;

use core::dev::{
    Device,
    Error,
    Result,
};
use core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use linux::libc as _libc;

/// [TAP interface](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
/// for sending and receiving raw ethernet frames.
pub struct Tap {
    tapfd: libc::c_int,
    max_transmission_unit: usize,
    ipv4_addr: Ipv4Address,
    eth_addr: EthernetAddress,
}

impl Tap {
    /// Creates or binds to an existing TAP interface with the specified IP and
    /// ethernet address.
    ///
    /// # Panics
    ///
    /// Causes a panic if [tun_alloc(...)](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
    /// runs into an error.
    pub fn new(ifr_name: &str, ipv4_addr: Ipv4Address, eth_addr: EthernetAddress) -> Tap {
        unsafe {
            let ifreq = _libc::c_ifreq::with_name(ifr_name);

            // Create the TAP...
            let tapfd = libc::open(
                "/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK,
            );

            if tapfd < 0 {
                panic!("Opening TAP: {}.", std::io::Error::last_os_error());
            }

            let mut _ifreq = ifreq.clone();
            _ifreq.ifr_ifru.ifr_flags = _libc::IFF_TAP | _libc::IFF_NO_PI;
            if libc::ioctl(tapfd, _libc::TUNSETIFF, &mut _ifreq as *mut _libc::c_ifreq) == -1 {
                panic!("TUNSETIFF TAP: {}.", std::io::Error::last_os_error());
            }

            // Query the MTU...
            let sockfd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);

            if sockfd == -1 {
                panic!("Opening socket: {}.", std::io::Error::last_os_error());
            }

            let mut _ifreq = ifreq.clone();

            if libc::ioctl(
                sockfd,
                _libc::SIOCGIFMTU,
                &mut _ifreq as *mut _libc::c_ifreq,
            ) == -1
            {
                panic!("IOCTL socket: {}.", std::io::Error::last_os_error());
            }

            libc::close(sockfd);

            let max_transmission_unit = _ifreq.ifr_ifru.ifr_mtu as usize;

            // Now we're done!
            Tap {
                tapfd,
                max_transmission_unit,
                ipv4_addr,
                eth_addr,
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
                return Err(Error::Busy);
            } else if wrote < 0 {
                Err(Error::IO(std::io::Error::last_os_error()))
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
                return Err(Error::Nothing);
            } else if read < 0 {
                Err(Error::IO(std::io::Error::last_os_error()))
            } else {
                Ok(read as usize)
            }
        }
    }

    fn max_transmission_unit(&self) -> usize {
        self.max_transmission_unit
    }

    fn ipv4_addr(&self) -> Ipv4Address {
        self.ipv4_addr
    }

    fn ethernet_addr(&self) -> EthernetAddress {
        self.eth_addr
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.tapfd);
        }
    }
}
