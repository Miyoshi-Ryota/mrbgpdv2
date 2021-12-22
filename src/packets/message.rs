use std::net::Ipv4Addr;

use bytes::BytesMut;

use crate::bgp_type::AutonomousSystemNumber;
use crate::error::{ConvertBgpMessageToBytesError, ConvertBytesToBgpMessageError};
use crate::packets::open::OpenMessage;

pub enum Message {
    Open(OpenMessage),
}

impl TryFrom<BytesMut> for Message {
    type Error = ConvertBytesToBgpMessageError;

    fn try_from(bytes: BytesMut) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl From<Message> for BytesMut {
    fn from(message: Message) -> BytesMut {
        match message {
            Message::Open(open) => open.into(),
        }
    }
}

impl Message {
    pub fn new_open(my_as_number: AutonomousSystemNumber, my_ip_addr: Ipv4Addr) -> Self {
        Self::Open(OpenMessage::new(my_as_number, my_ip_addr))
    }
}
