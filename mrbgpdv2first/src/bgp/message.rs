use super::timer::HoldTime;
use super::AutonomousSystemNumber;
use std::{convert::TryInto, net::Ipv4Addr};

#[derive(Debug)]
pub enum BgpMessage {
    Open(BgpOpenMessage),
    Keepalive(BgpKeepaliveMessage),
}

impl BgpMessage {
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            BgpMessage::Open(open) => open.serialize(),
            BgpMessage::Keepalive(keepalive) => keepalive.serialize(),
        }
    }

    pub fn deserialize(bytes: &Vec<u8>) -> Self {
        let bgp_type = BgpMessageType::from_type_number(bytes[18]);
        match bgp_type {
            BgpMessageType::Open => BgpMessage::Open(BgpOpenMessage::deserialize(bytes)),
            BgpMessageType::Keepalive => BgpMessage::Keepalive(BgpKeepaliveMessage::deserialize(bytes)),
        }
    }

    pub fn get_type(&self) -> BgpMessageType {
        match self {
            BgpMessage::Open(_) => BgpMessageType::Open,
            BgpMessage::Keepalive(_) => BgpMessageType::Keepalive,
        }
    }
}

#[derive(Debug)]
pub struct BgpMessageHeader {
    pub length: u16,
    pub type_: BgpMessageType,
}

impl BgpMessageHeader {
    fn serialize(&self) -> Vec<u8> {
        let marker = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let length = self.length.to_be_bytes();
        let type_ = self.type_.to_type_number();
        let mut bytes = vec![];
        bytes.append(&mut marker.to_vec());
        bytes.append(&mut length.to_vec());
        bytes.push(type_);
        bytes
    }

    pub fn deserialize(bytes: &Vec<u8>) -> Self {
        let length = u16::from_be_bytes(bytes[16..18].try_into().unwrap());
        let type_ = BgpMessageType::from_type_number(bytes[18]);
        Self { length, type_ }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BgpMessageType {
    Open,
    Keepalive,
}

impl BgpMessageType {
    fn to_type_number(&self) -> u8 {
        match self {
            BgpMessageType::Open => 1,
            BgpMessageType::Keepalive => 4,
        }
    }

    fn from_type_number(type_number: u8) -> Self {
        match type_number {
            1 => BgpMessageType::Open,
            4 => BgpMessageType::Keepalive,
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
struct BgpVersion(u8);

#[derive(Debug)]
pub struct BgpOpenMessage {
    header: BgpMessageHeader,
    version: BgpVersion,
    my_autonomous_system_number: AutonomousSystemNumber,
    hold_time: HoldTime,
    bgp_identifier: Ipv4Addr,
    optional_parameter_length: u8,
    // optional_parameterは実装・使用しないが、
    // 相手から受信したときに一応保存しておくためにプロパティとして用意している。
    optional_parameters: Vec<u8>,
}

impl BgpOpenMessage {
    pub fn new(my_as_number: AutonomousSystemNumber, my_ip_addr: Ipv4Addr) -> Self {
        let header = BgpMessageHeader {
            length: 29,
            type_: BgpMessageType::Open,
        };
        let version = BgpVersion(4);
        let my_autonomous_system_number = my_as_number;
        let hold_time = HoldTime(240);
        let bgp_identifier = my_ip_addr;
        let optional_parameter_length = 0;
        let optional_parameters = vec![];
        Self {
            header,
            version,
            my_autonomous_system_number,
            hold_time,
            bgp_identifier,
            optional_parameter_length,
            optional_parameters,
        }
    }
}

impl BgpOpenMessage {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.append(&mut self.header.serialize());
        bytes.push(self.version.0);
        bytes.append(&mut self.my_autonomous_system_number.0.to_be_bytes().to_vec());
        bytes.append(&mut self.hold_time.0.to_be_bytes().to_vec());
        bytes.append(&mut self.bgp_identifier.octets().to_vec());
        bytes.push(self.optional_parameter_length);
        bytes.append(&mut self.optional_parameters.clone());
        bytes
    }

    fn deserialize(bytes: &Vec<u8>) -> Self {
        let header = BgpMessageHeader::deserialize(&bytes[0..19].to_vec());
        let version = BgpVersion(bytes[19]);
        let my_autonomous_system_number =
            AutonomousSystemNumber(u16::from_be_bytes(bytes[20..22].try_into().unwrap()));
        let hold_time = HoldTime(u16::from_be_bytes(bytes[22..24].try_into().unwrap()));
        let bgp_identifier: Ipv4Addr = Ipv4Addr::new(bytes[24], bytes[25], bytes[26], bytes[27]);
        let optional_parameter_length = bytes[28];
        let optional_parameters = bytes[29..].to_vec();

        Self {
            header,
            version,
            my_autonomous_system_number,
            hold_time,
            bgp_identifier,
            optional_parameter_length,
            optional_parameters,
        }
    }
}

#[derive(Debug)]
pub struct BgpKeepaliveMessage {
    header: BgpMessageHeader,
}

impl BgpKeepaliveMessage {
    pub fn new() -> Self {
        let header = BgpMessageHeader {
            length: 19,
            type_: BgpMessageType::Keepalive,
        };
        Self { header }
    }

    fn serialize(&self) -> Vec<u8> {
        self.header.serialize()
    }

    fn deserialize(bytes: &Vec<u8>) -> Self {
        let header = BgpMessageHeader::deserialize(&bytes[0..19].to_vec());
        Self { header }
    }
}