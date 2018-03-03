use std::collections::HashMap;
use std::time::{
    Duration,
    Instant,
};

use core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use core::time::{
    Env,
    SystemEnv,
};

struct Entry {
    eth_addr: EthernetAddress,
    in_cache_since: Instant,
}

/// Maintains an expiring set of IPv4 -> ethernet address mappings.
pub struct ArpCache<T = SystemEnv>
where
    T: Env,
{
    entries: HashMap<Ipv4Address, Entry>,
    expiration: Duration,
    in_cache_since_min: Instant,
    time_env: T,
}

impl<T: Env> ArpCache<T> {
    /// Creates an ARP cache where ethernet address mappings expire after
    /// expiration_in_secs seconds.
    pub fn new(expiration_in_secs: u64, time_env: T) -> ArpCache<T> {
        ArpCache {
            entries: HashMap::new(),
            expiration: Duration::from_secs(expiration_in_secs),
            in_cache_since_min: Instant::now(),
            time_env: time_env,
        }
    }

    /// Lookup the ethernet address for an IPv4 address.
    pub fn eth_addr_for_ip(&mut self, ipv4_addr: Ipv4Address) -> Option<EthernetAddress> {
        self.expire_eth_addr();

        match self.entries.get(&ipv4_addr) {
            Some(entry) => Some(entry.eth_addr),
            _ => None,
        }
    }

    /// Create or update the ethernet address mapping for an IPv4 address.
    pub fn set_eth_addr_for_ip(&mut self, ipv4_addr: Ipv4Address, eth_addr: EthernetAddress) {
        self.expire_eth_addr();

        let in_cache_since = self.time_env.now_instant();

        if self.entries.len() == 0 {
            self.in_cache_since_min = in_cache_since;
        }

        self.entries.insert(
            ipv4_addr,
            Entry {
                eth_addr,
                in_cache_since,
            },
        );
    }

    /// Purge Ethernet address entries translations that have expired.
    fn expire_eth_addr(&mut self) {
        let now = self.time_env.now_instant();

        if now > self.in_cache_since_min + self.expiration {
            // Purge expired entries...
            let expiration = self.expiration;
            self.entries
                .retain(|_, entry| now.duration_since(entry.in_cache_since) <= expiration);

            // Update timestamp of the oldest entry...
            let in_cache_since = self.entries.iter().map(|(_, entry)| entry.in_cache_since);
            self.in_cache_since_min = match in_cache_since.min() {
                Some(in_cache_since) => in_cache_since,
                None => now,
            }
        }
    }

    #[cfg(test)]
    fn time_env(&mut self) -> &mut T {
        &mut self.time_env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::MockEnv;

    fn arp_cache() -> ArpCache<MockEnv> {
        ArpCache::new(60, MockEnv::new())
    }

    fn ipv4(i: u8) -> Ipv4Address {
        Ipv4Address::new([0, 0, 0, i])
    }

    fn eth(i: u8) -> EthernetAddress {
        EthernetAddress::new([0, 0, 0, 0, 0, i])
    }

    #[test]
    fn test_lookup_ip_with_no_mapping() {
        let mut arp_cache = arp_cache();
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(0)), None);
    }

    #[test]
    fn test_lookup_ip_with_mapping() {
        let mut arp_cache = arp_cache();

        arp_cache.set_eth_addr_for_ip(ipv4(0), eth(0));
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));

        arp_cache.time_env().now += Duration::from_secs(60);
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));
    }

    #[test]
    fn test_lookup_ip_after_expiring() {
        let mut arp_cache = arp_cache();

        arp_cache.set_eth_addr_for_ip(ipv4(0), eth(0));
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));

        arp_cache.time_env().now += Duration::from_secs(61);
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(0)), None);
    }

    #[test]
    fn test_push_back_expiration() {
        let mut arp_cache = arp_cache();

        arp_cache.set_eth_addr_for_ip(ipv4(0), eth(0));
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));

        arp_cache.time_env().now += Duration::from_secs(60);
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));

        arp_cache.set_eth_addr_for_ip(ipv4(0), eth(0));
        arp_cache.time_env().now += Duration::from_secs(60);

        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));

        arp_cache.time_env().now += Duration::from_secs(1);
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(0)), None);
    }

    #[test]
    fn test_chained_expiration() {
        let mut arp_cache = arp_cache();

        arp_cache.set_eth_addr_for_ip(ipv4(0), eth(0));
        arp_cache.time_env().now += Duration::from_secs(30);
        arp_cache.set_eth_addr_for_ip(ipv4(1), eth(1));
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(0)).unwrap(), eth(0));
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(1)).unwrap(), eth(1));

        arp_cache.time_env().now += Duration::from_secs(31);
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(0)), None);
        assert_eq!(arp_cache.eth_addr_for_ip(ipv4(1)).unwrap(), eth(1));

        arp_cache.time_env().now += Duration::from_secs(30);
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(0)), None);
        assert_matches!(arp_cache.eth_addr_for_ip(ipv4(1)), None);
    }
}
