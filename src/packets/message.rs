use bytes::BytesMut;

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
        todo!();
    }
}
