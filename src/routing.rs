use std::net::{IpAddr, Ipv4Addr};

use crate::config::Config;
use crate::path_attribute::{AsPath, Origin, PathAttribute};
use anyhow::{Context, Result};
use futures::stream::{Next, TryStreamExt};
use ipnetwork::Ipv4Network;
use rtnetlink::{new_connection, Handle, IpVersion};

#[derive(Debug, PartialEq, Eq, Clone)]
struct LocRib(Vec<RibEntry>);

impl LocRib {
    async fn new(config: &Config) -> Result<Self> {
        let path_attributes = vec![
            PathAttribute::Origin(Origin::Igp),
            // AS Pathは、ほかのピアから受信したルートと統一的に扱うために、
            // LocRib -> AdjRibOutにルートを送るときに、自分のAS番号を
            // 追加するので、ここでは空にしておく。
            PathAttribute::AsPath(AsPath::AsSequence(vec![])),
            PathAttribute::NextHop(config.local_ip),
        ];

        let mut rib = vec![];
        for network in &config.networks {
            let routes = Self::lookup_kernel_routing_table(*network).await?;
            for route in routes {
                rib.push(RibEntry {
                    network_address: route,
                    path_attributes: path_attributes.clone(),
                })
            }
        }
        Ok(Self(rib))
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

#[derive(Debug, PartialEq, Eq, Clone)]
struct RibEntry {
    network_address: Ipv4Network,
    path_attributes: Vec<PathAttribute>,
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
