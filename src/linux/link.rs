use std;

use libc;

use core::link::{
    Error,
    EthernetLink,
    Ipv4Link,
    Link,
    Result,
};
use core::repr::{
    Ipv4,
    Mac,
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
    /// Causes a panic if any of the operations in [tun_alloc]
    /// (https://www.kernel.org/doc/Documentation/networking/tuntap.txt) error.
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

    fn inet_ioctl(request: libc::c_ulong, ifreq: &mut _libc::c_ifreq) -> Result<()> {
        unsafe {
            let fd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);

            if fd == -1 {
                return Err(Error::IO(std::io::Error::last_os_error()));
            }

            if libc::ioctl(fd, request, ifreq as *mut _libc::c_ifreq) == -1 {
                libc::close(fd);
                return Err(Error::IO(std::io::Error::last_os_error()));
            }

            libc::close(fd);
            Ok(())
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
            let mut ifreq = self.ifreq.clone();
            Self::inet_ioctl(_libc::SIOCGIFMTU, &mut ifreq)?;
            Ok(ifreq.ifr_ifru.ifr_mtu as usize)
        }
    }
}

impl EthernetLink for Tap {
    fn get_ethernet_addr(&self) -> Result<Mac> {
        unsafe {
            let mut ifreq = self.ifreq.clone();
            Self::inet_ioctl(_libc::SIOCGIFHWADDR, &mut ifreq)?;

            let c_addr = &ifreq.ifr_ifru.ifr_addr;
            if c_addr.sa_family != _libc::ARPHRD_ETHER as u16 {
                return Err(Error::Unknown("Ethernet address not found."));
            }

            let mut buffer = [0 as u8; 6];
            for i in 0..6 {
                buffer[i] = c_addr.sa_data[i] as u8;
            }

            Ok(Mac::new(buffer))
        }
    }
}

impl Ipv4Link for Tap {
    fn get_ipv4_addr(&self) -> Result<Ipv4> {
        unsafe {
            let mut ifreq = self.ifreq.clone();
            Self::inet_ioctl(_libc::SIOCGIFADDR, &mut ifreq)?;

            let c_addr = &ifreq.ifr_ifru.ifr_addr;
            if c_addr.sa_family != libc::AF_INET as u16 {
                return Err(Error::Unknown("Ipv4 address not found."));
            }

            let c_addr_in = std::mem::transmute::<&libc::sockaddr, &libc::sockaddr_in>(c_addr);
            let buffer: [u8; 4] = std::mem::transmute(c_addr_in.sin_addr.s_addr);

            Ok(Ipv4::new(buffer))
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
