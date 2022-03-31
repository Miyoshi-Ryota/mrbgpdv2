use std::collections::HashMap;
use std::net::Ipv4Addr;

use crate::routing::Ipv4Network;
use anyhow::Context;
use bytes::{BufMut, BytesMut};

use crate::error::ConvertBytesToBgpMessageError;
use crate::packets::header::Header;
use crate::path_attribute::{AsPath, Origin, PathAttribute};
use crate::routing::{AdjRibOut, RibEntry};

use super::header::MessageType;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct UpdateMessage {
    header: Header,
    pub withdrawn_routes: Vec<Ipv4Network>,
    withdrawn_routes_length: u16, // ルート数ではなく、bytesにしたときのオクテット数。
    pub path_attributes: Vec<PathAttribute>,
    path_attributes_length: u16, // bytesにした時のオクテット数。
    pub network_layer_reachability_information: Vec<Ipv4Network>,
    // NLRIのオクテット数はBGP UpdateMessageに含めず、
    // Headerのサイズを計算することにしか使用しないため、
    // メンバに含めていない。
}

impl UpdateMessage {
    fn new(
        path_attributes: Vec<PathAttribute>,
        network_layer_reachability_information: Vec<Ipv4Network>,
        withdrawn_routes: Vec<Ipv4Network>,
    ) -> Self {
        let path_attributes_length =
            path_attributes.iter().map(|p| p.bytes_len()).sum::<usize>() as u16;
        let network_layer_reachability_information_length = network_layer_reachability_information
            .iter()
            .map(|r| r.bytes_len())
            .sum::<usize>() as u16;
        let withdrawn_routes_length = withdrawn_routes
            .iter()
            .map(|w| w.bytes_len())
            .sum::<usize>() as u16;
        let header_minimum_length: u16 = 19;
        let header = Header::new(
            header_minimum_length
                + path_attributes_length
                + network_layer_reachability_information_length
                + withdrawn_routes_length
                + 4, // path_attributes_length(u16)とwithdrawn_routes_length(u16)のbytes表現分,
            MessageType::Update,
        );
        Self {
            header,
            withdrawn_routes,
            withdrawn_routes_length,
            path_attributes,
            path_attributes_length,
            network_layer_reachability_information,
        }
    }
}

impl From<UpdateMessage> for BytesMut {
    fn from(message: UpdateMessage) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put::<BytesMut>(message.header.into());
        bytes.put_u16(message.withdrawn_routes_length);
        message
            .withdrawn_routes
            .iter()
            .for_each(|r| bytes.put::<BytesMut>(r.into()));

        bytes.put_u16(message.path_attributes_length);
        message
            .path_attributes
            .iter()
            .for_each(|r| bytes.put::<BytesMut>(r.into()));

        message
            .network_layer_reachability_information
            .iter()
            .for_each(|r| bytes.put::<BytesMut>(r.into()));
        bytes
    }
}

impl TryFrom<BytesMut> for UpdateMessage {
    type Error = ConvertBytesToBgpMessageError;
    fn try_from(bytes: BytesMut) -> Result<Self, Self::Error> {
        let header = Header::try_from(BytesMut::from(&bytes[0..19]))?;
        let withdrawn_routes_length: u16 =
            u16::from_be_bytes(bytes[19..21].try_into().context(format!(
                "Bytes: {:?}からwithdrawn_routes_lengthに変換できませんでした",
                &bytes
            ))?);
        let withdrawn_routes_end_index = 21 + withdrawn_routes_length as usize;
        let withdrawn_routes_bytes = &bytes[21..withdrawn_routes_end_index];
        let withdrawn_routes = Ipv4Network::from_u8_slice(withdrawn_routes_bytes)?;

        let path_attributes_start_index = withdrawn_routes_end_index + 2;
        let total_path_attribute_length = u16::from_be_bytes(
            bytes[withdrawn_routes_end_index..path_attributes_start_index]
                .try_into()
                .context(format!(
                    "Bytes: {:?}からtotal_path_attribute_lengthに変換できませんでした",
                    &bytes
                ))?,
        );

        let path_attributes_bytes = &bytes[path_attributes_start_index
            ..path_attributes_start_index + total_path_attribute_length as usize];
        let path_attributes = PathAttribute::from_u8_slice(path_attributes_bytes)?;
        let nlri_start_index = path_attributes_start_index + total_path_attribute_length as usize;
        let network_layer_reachability_information =
            Ipv4Network::from_u8_slice(&bytes[nlri_start_index..])?;

        Ok(Self {
            header,
            withdrawn_routes_length,
            withdrawn_routes,
            path_attributes_length: total_path_attribute_length,
            path_attributes,
            network_layer_reachability_information,
        })
    }
}

/// AdjRibOutからUpdateMessageに変換する。
/// PathAttributeごとにUpdateMessageが分かれるためVec<UpdateMessage>の戻り値にしている。
impl From<&AdjRibOut> for Vec<UpdateMessage> {
    fn from(rib: &AdjRibOut) -> Self {
        let mut hash_map: HashMap<Vec<PathAttribute>, Vec<Ipv4Network>> = HashMap::new();
        for entry in rib.0.routes() {
            if let Some(routes) = hash_map.get_mut(&entry.path_attributes) {
                routes.push(entry.network_address);
            } else {
                hash_map.insert(entry.path_attributes.clone(), vec![entry.network_address]);
            }
        }

        let mut updates = vec![];
        for (path_attributes, routes) in hash_map.into_iter() {
            // ToDo: withdrawn routesに対応する。
            updates.push(UpdateMessage::new(path_attributes, routes, vec![]));
        }
        updates
    }
}

#[cfg(test)]
mod tests {
    use crate::routing::Rib;

    use super::*;

    async fn update_message_from_adj_rib_out() {
        // 本テストの値は環境によって異なる。
        // 本実装では開発機, テスト実施機に10.200.100.0/24に属するIPが付与されていることを仮定している。
        // docker-composeした環境のhost2で実行することを仮定している。

        let path_attributes = vec![
            PathAttribute::Origin(Origin::Igp),
            PathAttribute::AsPath(AsPath::AsSequence(vec![64513.into()])),
            PathAttribute::NextHop("10.200.100.3".parse().unwrap()),
        ];

        let mut rib = Rib::new();
        vec![RibEntry {
            network_address: "10.100.220.0/24".parse().unwrap(),
            path_attributes: path_attributes.clone(),
        }]
        .into_iter()
        .map(|e| rib.insert(e));
        let adj_rib_out = AdjRibOut(rib);
        let expected_update_message = UpdateMessage::new(
            path_attributes,
            vec!["10.100.220.0/24".parse().unwrap()],
            vec![],
        );
        assert_eq!(
            Vec::<UpdateMessage>::from(&adj_rib_out),
            vec![expected_update_message]
        );
    }
}
