use bytes::BytesMut;
use crate::routing::Ipv4Network;

use crate::packets::header::Header;
use crate::path_attribute::{AsPath, Origin, PathAttribute};
use crate::routing::{AdjRibOut, RibEntry};
use crate::error::ConvertBytesToBgpMessageError;

use super::header::MessageType;


#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct UpdateMessage {
    header: Header,
    withdrawn_routes: Vec<Ipv4Network>,
    path_attributes: Vec  <PathAttribute>,
    network_layer_reachability_information: Vec<Ipv4Network>,
}

impl UpdateMessage {
    fn new(path_attributes: Vec<PathAttribute>, network_layer_reachability_information: Vec<Ipv4Network>, withdrawn_routes: Vec<Ipv4Network>) -> Self {
        todo!();
    }
}

impl From<UpdateMessage> for BytesMut {
    fn from(_: UpdateMessage) -> Self {
        todo!();
    }
}

impl TryFrom<BytesMut> for UpdateMessage {
    type Error = ConvertBytesToBgpMessageError;
    fn try_from(value: BytesMut) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl From<&AdjRibOut> for Vec<UpdateMessage> {
    fn from(rib: &AdjRibOut) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
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
        let adj_rib_out = AdjRibOut(vec![RibEntry {
            network_address: "10.100.220.0/24".parse().unwrap(),
            path_attributes: path_attributes.clone(),
        }]);
        let expected_update_message = UpdateMessage::new(
            path_attributes,
            vec!["10.100.220.0/24".parse().unwrap()],
            vec![]);
        assert_eq!(Vec::<UpdateMessage>::from(&adj_rib_out), vec![expected_update_message]);
    }
}
