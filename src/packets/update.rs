use bytes::BytesMut;
use crate::routing::Ipv4Network;

use crate::packets::header::Header;
use crate::path_attribute::PathAttribute;
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

