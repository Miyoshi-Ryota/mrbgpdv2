use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use crate::bgp_type::AutonomousSystemNumber;
use crate::config::Config;
use crate::error::{ConfigParseError, ConstructIpv4NetworkError, ConvertBytesToBgpMessageError};
use crate::packets::update::UpdateMessage;
use crate::path_attribute::{AsPath, Origin, PathAttribute};
use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use futures::stream::{Next, TryStreamExt};
use rtnetlink::{new_connection, Handle, IpVersion};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub struct Ipv4Network(ipnetwork::Ipv4Network);

impl Deref for Ipv4Network {
    type Target = ipnetwork::Ipv4Network;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Ipv4Network {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<ipnetwork::Ipv4Network> for Ipv4Network {
    fn from(ip_network: ipnetwork::Ipv4Network) -> Self {
        Self(ip_network)
    }
}

impl From<&Ipv4Network> for BytesMut {
    fn from(network: &Ipv4Network) -> BytesMut {
        let prefix = network.prefix();

        let n = network.network().octets();
        let network_bytes = match prefix {
            0 => vec![],
            1..9 => n[0..1].into(),
            9..17 => n[0..2].into(),
            17..25 => n[0..3].into(),
            25..33 => n[0..4].into(),
            _ => panic!("prefixが0..32の間ではありません！"),
        };
        let mut bytes = BytesMut::new();
        bytes.put_u8(prefix);
        bytes.put(&network_bytes[..]);
        bytes
    }
}

impl FromStr for Ipv4Network {
    type Err = ConfigParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let network = s
            .parse::<ipnetwork::Ipv4Network>()
            .context("s: {:?}を、Ipv4Networkにparse出来ませんでした")?;
        Ok(Self(network))
    }
}

impl Ipv4Network {
    pub fn bytes_len(&self) -> usize {
        match self.prefix() {
            0 => 1,
            1..9 => 2,
            9..17 => 3,
            17..25 => 4,
            25..33 => 5,
            _ => panic!("prefixが0..32の間ではありません！"),
        }
    }

    pub fn new(addr: Ipv4Addr, prefix: u8) -> Result<Self, ConstructIpv4NetworkError> {
        let net = ipnetwork::Ipv4Network::new(addr, prefix).context(format!(
            "Ipv4NetworkをConstruct出来ませんでした。addr: {}, prefix: {}",
            addr, prefix
        ))?;
        Ok(Self(net))
    }

    /// 本来、From Traitを実装するべきだと思うけれど、
    /// Vec<..>に実装するのが、New Type Patternが必要になり
    /// 大変なので変な関連関数を追加することで対応した。
    pub fn from_u8_slice(bytes: &[u8]) -> Result<Vec<Self>, ConvertBytesToBgpMessageError> {
        let mut networks = vec![];
        let mut i = 0;
        while bytes.len() > i {
            let prefix = bytes[i];
            i += 1;
            if prefix == 0 {
                networks.push(Ipv4Network::new(Ipv4Addr::new(0, 0, 0, 0), prefix).context("")?);
            } else if (1..=8).contains(&prefix) {
                networks
                    .push(Ipv4Network::new(Ipv4Addr::new(bytes[i], 0, 0, 0), prefix).context("")?);
                i += 1;
            } else if (9..=16).contains(&prefix) {
                networks.push(
                    Ipv4Network::new(Ipv4Addr::new(bytes[i], bytes[i + 1], 0, 0), prefix)
                        .context("bytes -> Ipv4に変換出来ませんでした。")?,
                );
                i += 2;
            } else if (17..=24).contains(&prefix) {
                networks.push(
                    Ipv4Network::new(
                        Ipv4Addr::new(bytes[i], bytes[i + 1], bytes[i + 2], 0),
                        prefix,
                    )
                    .context("bytes -> Ipv4に変換出来ませんでした。")?,
                );
                i += 3;
            } else if (24..=32).contains(&prefix) {
                networks.push(
                    Ipv4Network::new(
                        Ipv4Addr::new(bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]),
                        prefix,
                    )
                    .context("bytes -> Ipv4に変換出来ませんでした。")?,
                );
                i += 4;
            } else {
                return Err(ConvertBytesToBgpMessageError::from(anyhow::anyhow!(
                    "bytes -> Ipv4Networkに変換が出来ませんでした。Prefixが0-32の間ではありません。"
                )));
            };
        }
        Ok(networks)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum RibEntryStatus {
    New,
    UnChanged,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Rib(HashMap<RibEntry, RibEntryStatus>);
impl Rib {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn insert(&mut self, entry: RibEntry) {
        self.0.entry(entry).or_insert(RibEntryStatus::New);
    }

    pub fn update_to_all_unchanged(&mut self) {
        self.0
            .iter_mut()
            .for_each(|(_, v)| *v = RibEntryStatus::UnChanged);
    }

    pub fn routes(&self) -> Keys<'_, RibEntry, RibEntryStatus> {
        self.0.keys()
    }

    pub fn does_contain_new_route(&self) -> bool {
        self.0.values().map(|v| &RibEntryStatus::New == v).any(|v| v)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocRib {
    pub rib: Rib,
    local_as_number: AutonomousSystemNumber,
}

impl LocRib {
    pub async fn new(config: &Config) -> Result<Self> {
        let path_attributes = vec![
            PathAttribute::Origin(Origin::Igp),
            // AS Pathは、ほかのピアから受信したルートと統一的に扱うために、
            // LocRib -> AdjRibOutにルートを送るときに、自分のAS番号を
            // 追加するので、ここでは空にしておく。
            PathAttribute::AsPath(AsPath::AsSequence(vec![])),
            PathAttribute::NextHop(config.local_ip),
        ];

        let mut rib = Rib::new();
        for network in &config.networks {
            let routes = Self::lookup_kernel_routing_table(*network).await?;
            for route in routes {
                rib.insert(RibEntry {
                    network_address: route,
                    path_attributes: path_attributes.clone(),
                })
            }
        }
        Ok(Self {
            rib,
            local_as_number: config.local_as,
        })
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
                ipnetwork::Ipv4Network::new(addr, prefix)?.into()
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

    /// AdjRibInから必要なルートをインストールする。
    /// この時、自ASが含まれているルートはインストールしない。
    /// 参考: 9.1.2.  Phase 2: Route Selection in RFC4271.
    pub fn install_from_adj_rib_in(&mut self, adj_rib_in: &AdjRibIn) {
        adj_rib_in
            .0
            .routes()
            .filter(|entry| !entry.does_contain_as(self.local_as_number))
            .for_each(|entry| self.rib.insert(entry.clone()));
    }

    pub async fn write_to_kernel_routing_table(&self) -> Result<()> {
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        for e in self.rib.routes() {
            for p in &e.path_attributes {
                if let PathAttribute::NextHop(gateway) = p {
                    let dest = e.network_address;
                    handle
                        .route()
                        .add()
                        .v4()
                        .destination_prefix(dest.ip(), dest.prefix())
                        .gateway(*gateway)
                        .execute()
                        .await?;
                    break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AdjRibOut(pub Rib);

impl AdjRibOut {
    pub fn new() -> Self {
        Self(Rib::new())
    }

    pub fn install_from_loc_rib(&mut self, loc_rib: &LocRib, config: &Config) {
        for r in loc_rib.rib.routes() {
            let mut route = r.clone();
            route.append_as_path(config.local_as);
            route.change_next_hop(config.local_ip);
            self.0.insert(route);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AdjRibIn(pub Rib);
impl AdjRibIn {
    pub fn new() -> Self {
        Self(Rib::new())
    }
    pub fn install_from_update(&mut self, update: UpdateMessage, config: &Config) {
        // ToDo: * rib_entryが重複しないようにする
        //       * withdrawnに対応する。
        let path_attributes = update.path_attributes;
        for network in update.network_layer_reachability_information {
            let rib_entry = RibEntry {
                network_address: network,
                path_attributes: path_attributes.clone(),
            };
            // PathAttributesが変わってたらインストールする必要がある。
            self.0.insert(rib_entry);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct RibEntry {
    pub network_address: Ipv4Network,
    pub path_attributes: Vec<PathAttribute>,
}

impl RibEntry {
    fn append_as_path(&mut self, as_number: AutonomousSystemNumber) {
        for path_attribute in &mut self.path_attributes {
            if let PathAttribute::AsPath(as_path) = path_attribute {
                as_path.add(as_number)
            };
        }
    }

    fn change_next_hop(&mut self, next_hop: Ipv4Addr) {
        for path_attribute in &mut self.path_attributes {
            if let PathAttribute::NextHop(addr) = path_attribute {
                *addr = next_hop;
            }
        }
    }

    fn does_contain_as(&self, as_number: AutonomousSystemNumber) -> bool {
        for path_attribute in &self.path_attributes {
            if let PathAttribute::AsPath(as_path) = path_attribute {
                return as_path.does_contain(as_number);
            }
        }
        false
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
        let network = ipnetwork::Ipv4Network::new("10.200.100.0".parse().unwrap(), 24)
            .unwrap()
            .into();
        let routes = LocRib::lookup_kernel_routing_table(network).await.unwrap();
        let expected = vec![network];
        assert_eq!(routes, expected);
    }

    #[tokio::test]
    async fn loc_rib_to_adj_rib_out() {
        // 本テストの値は環境によって異なる。
        // 本実装では開発機, テスト実施機に10.200.100.0/24に属するIPが付与されていることを仮定している。
        // docker-composeした環境のhost2で実行することを仮定している。
        let config: Config = "64513 10.200.100.3 64512 10.200.100.2 passive 10.100.220.0/24"
            .parse()
            .unwrap();
        let mut loc_rib = LocRib::new(&config).await.unwrap();
        let mut adj_rib_out = AdjRibOut::new();
        adj_rib_out.install_from_loc_rib(&mut loc_rib, &config);

        println!("adj_rib_out is created!");
        println!("expected_adj_rib_out is creating!");
        let mut rib = Rib::new();
        rib.insert(RibEntry {
            network_address: "10.100.220.0/24".parse().unwrap(),
            path_attributes: vec![
                PathAttribute::Origin(Origin::Igp),
                PathAttribute::AsPath(AsPath::AsSequence(vec![64513.into()])),
                PathAttribute::NextHop("10.200.100.3".parse().unwrap()),
            ],
        });
        let expected_adj_rib_out = AdjRibOut(rib);

        assert_eq!(adj_rib_out, expected_adj_rib_out);
    }
}
