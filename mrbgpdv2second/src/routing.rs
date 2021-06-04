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
