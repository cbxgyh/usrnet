use libc;

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/if_tun.h
pub const IFF_TAP: libc::c_short = 0x0002;

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/if_tun.h
pub const IFF_NO_PI: libc::c_short = 0x1000;

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/if_tun.h
pub const TUNSETIFF: libc::c_ulong = 0x400454CA;

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/sockios.h
pub const SIOCGIFMTU: libc::c_ulong = 0x8921;

#[repr(C)]
#[derive(Clone, Copy)]
// https://linux.die.net/man/7/netdevice
pub struct c_ifreq {
    pub ifr_name: [libc::c_char; libc::IF_NAMESIZE],
    pub ifr_ifru: c_ifreq_ifru,
}

impl c_ifreq {
    pub fn with_name(ifr_name: &str) -> c_ifreq {
        assert!(ifr_name.len() <= libc::IF_NAMESIZE);

        let mut ifreq = c_ifreq {
            ifr_name: [0; libc::IF_NAMESIZE],
            ifr_ifru: c_ifreq_ifru { ifr_flags: 0 },
        };

        for (i, c) in ifr_name.as_bytes().iter().enumerate() {
            ifreq.ifr_name[i] = *c as libc::c_char;
        }

        ifreq
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union c_ifreq_ifru {
    pub ifr_flags: libc::c_short,
    pub ifr_mtu: libc::c_int,
}

pub fn errno() -> libc::c_int {
    unsafe {
        let errno = libc::__errno_location();
        *errno
    }
}
