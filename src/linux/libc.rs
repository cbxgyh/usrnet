use libc;

pub const IFF_TAP: libc::c_int = 0x0002;

pub const IFF_NO_PI: libc::c_int = 0x1000;

pub const ETH_P_ALL: libc::c_int = 0x0003;

pub const TUNSETIFF: libc::c_ulong = 0x400454CA;

pub const SIOCGIFMTU: libc::c_ulong = 0x8921;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// [https://linux.die.net/man/7/netdevice](https://linux.die.net/man/7/netdevice)
pub struct c_ifreq {
    pub ifr_name: [libc::c_char; libc::IF_NAMESIZE],
    pub ifr_data: libc::c_int, // ifr_flags and ifr_mtu
}

impl c_ifreq {
    pub fn with_name(ifr_name: &str) -> c_ifreq {
        assert!(ifr_name.len() <= libc::IF_NAMESIZE);

        let mut ifreq = c_ifreq {
            ifr_name: [0; libc::IF_NAMESIZE],
            ifr_data: 0,
        };

        for (i, c) in ifr_name.as_bytes().iter().enumerate() {
            ifreq.ifr_name[i] = *c as libc::c_char;
        }

        ifreq
    }
}
