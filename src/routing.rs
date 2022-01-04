use std::net::{IpAddr, Ipv4Addr};

use crate::config::Config;
use anyhow::{Context, Result};
use futures::stream::{Next, TryStreamExt};
use ipnetwork::Ipv4Network;
use rtnetlink::{new_connection, Handle, IpVersion};

#[derive(Debug, PartialEq, Eq, Clone)]
struct LocRib;

impl LocRib {
    async fn new(config: &Config) -> Self {
        todo!();
    }

    async fn lookup_kernel_routing_table(
        network_address: Ipv4Network,
    ) -> Result<(Vec<Ipv4Network>)> {
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        let mut routes = handle.route().get(IpVersion::V4).execute();
        let mut results = vec![];
        while let Some(route) = routes.try_next().await? {
            let destination = if let Some((IpAddr::V4(addr), prefix)) = route.destination_prefix() {
                Ipv4Network::new(addr, prefix)?
            } else {
                continue;
            };

            if destination != network_address {
                continue;
            }

            results.push(destination);
        }
        Ok(results)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn loclib_can_lookup_routing_table() {
        // 本テストの値は環境によって異なる。
        // 本実装では開発機, テスト実施機に10.200.100.0/24に属するIPが付与されていることを仮定している。
        let network = Ipv4Network::new("10.200.100.0".parse().unwrap(), 24).unwrap();
        let routes = LocRib::lookup_kernel_routing_table(network).await.unwrap();
        let expected = vec![network];
        assert_eq!(routes, expected);
    }
}
