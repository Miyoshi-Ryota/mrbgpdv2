use futures::TryStreamExt;
use rtnetlink::packet::RouteMessage;
use rtnetlink::{new_connection, IpVersion};
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::str::FromStr;
use tokio::runtime::Runtime;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct RoutingTableEntry {
    network_address: IpPrefix,
    nexthop: Nexthop,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Nexthop {
    Ipv4Addr(Ipv4Addr),
    DirectConected,
}

impl RoutingTableEntry {
    fn new(network_address: IpPrefix, nexthop: Nexthop) -> Self {
        if !network_address.is_network_address() {
            panic!(
                "This address of routing table entry is
                    not network address: {:?}",
                network_address
            );
        }
        Self {
            network_address,
            nexthop,
        }
    }

    fn from_route_message(route_message: &RouteMessage) -> Self {
        let (dest_addr, prefix) = match route_message.destination_prefix() {
            Some((dest_addr, prefix)) => (dest_addr, prefix),
            None => (IpAddr::V4("0.0.0.0".parse().unwrap()), 0),
        };
        let network_address = match dest_addr {
            IpAddr::V4(ip_addr) => IpPrefix { ip_addr, prefix },
            _ => unimplemented!(),
        };

        let nexthop = if route_message.gateway().is_some() {
            match route_message.gateway().unwrap() {
                IpAddr::V4(ip_addr) => Nexthop::Ipv4Addr(ip_addr),
                _ => unimplemented!(),
            }
        } else if route_message.output_interface().is_some() {
            Nexthop::DirectConected
        } else {
            panic!()
        };

        Self {
            network_address,
            nexthop,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct IpPrefix {
    ip_addr: Ipv4Addr,
    prefix: u8,
}

impl IpPrefix {
    fn is_network_address(&self) -> bool {
        let netmask = self.get_subnet_netmask();
        let ip_addr_bit: u32 = self.ip_addr.into();
        netmask | ip_addr_bit == netmask
    }

    fn does_include(&self, other: &IpPrefix) -> bool {
        if self.prefix > other.prefix {
            return false;
        }
        let self_netmask = self.get_subnet_netmask();
        let self_ip_addr_bit: u32 = self.ip_addr.into();
        let other_ip_addr_bit: u32 = other.ip_addr.into();
        self_netmask & other_ip_addr_bit == self_ip_addr_bit
    }

    fn get_subnet_netmask(&self) -> u32 {
        let mut netmask: u32 = 0;
        for i in 0..self.prefix {
            netmask += 1 << i;
        }
        netmask << (32 - self.prefix)
    }
}

#[derive(Debug)]
struct ParseIpPrefixError;

impl FromStr for IpPrefix {
    type Err = ParseIpPrefixError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: Vec<&str> = s.split('/').collect();
        let ip_addr: Ipv4Addr = s[0].parse().or(Err(ParseIpPrefixError))?;
        let prefix: u8 = s[1].parse().or(Err(ParseIpPrefixError))?;
        Ok(IpPrefix { ip_addr, prefix })
    }
}

fn lookup_routing_table(lookup_addr: &IpPrefix) -> Vec<RoutingTableEntry> {
    let mut route_messages: Vec<RouteMessage> = vec![];
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);
        let mut routes = handle.route().get(IpVersion::V4).execute();
        while let Some(route) = routes.try_next().await.unwrap() {
            route_messages.push(route);
        }
    });
    route_messages
        .iter()
        .map(|m| RoutingTableEntry::from_route_message(m))
        .filter(|r| lookup_addr.does_include(&r.network_address))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_lookup_routing_table() {
        init();
        let lookup_addr: IpPrefix = "10.0.2.0/24".parse().unwrap();
        let routing_entries: HashSet<RoutingTableEntry> =
            lookup_routing_table(&lookup_addr).into_iter().collect();
        let expected_routing_entries: HashSet<RoutingTableEntry> = vec![
            RoutingTableEntry::new("10.0.2.0/24".parse().unwrap(), Nexthop::DirectConected),
            RoutingTableEntry::new("10.0.2.2/32".parse().unwrap(), Nexthop::DirectConected),
            RoutingTableEntry::new("10.0.2.0/32".parse().unwrap(), Nexthop::DirectConected),
            RoutingTableEntry::new("10.0.2.255/32".parse().unwrap(), Nexthop::DirectConected),
            RoutingTableEntry::new("10.0.2.15/32".parse().unwrap(), Nexthop::DirectConected),
        ]
        .into_iter()
        .collect();
        let difference: HashSet<RoutingTableEntry> = routing_entries
            .symmetric_difference(&expected_routing_entries)
            .cloned()
            .collect();
        assert!(difference.is_empty());
    }
}
