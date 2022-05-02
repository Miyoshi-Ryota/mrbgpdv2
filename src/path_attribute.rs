use anyhow::Context;
use bytes::{BufMut, BytesMut};

use crate::{
    bgp_type::AutonomousSystemNumber, error::ConvertBytesToBgpMessageError,
};
use std::{collections::BTreeSet, net::Ipv4Addr};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum PathAttribute {
    Origin(Origin),
    AsPath(AsPath),
    NextHop(Ipv4Addr),
    DontKnow(Vec<u8>), // 対応してないPathAttribute用
}

impl PathAttribute {
    pub fn bytes_len(&self) -> usize {
        let path_attribute_value_length = match self {
            PathAttribute::Origin(o) => 1,
            PathAttribute::AsPath(a) => a.bytes_len(),
            PathAttribute::NextHop(_) => 4,
            PathAttribute::DontKnow(v) => v.len(),
        };
        // flagを表すoctet, typeを表すoctet分を追加。
        let length = path_attribute_value_length + 2;
        if path_attribute_value_length > 255 {
            length + 2 // path_attribute_value_lengthが255以上のとき、
                       // attribute lengthを表すoctetが1 octetで表せず
                       // 2octetsになる。
        } else {
            length + 1 // attribute lengthを表すoctet分追加。
        }
    }

    pub fn from_u8_slice(
        bytes: &[u8],
    ) -> Result<Vec<PathAttribute>, ConvertBytesToBgpMessageError> {
        let mut path_attributes = vec![];
        let mut i = 0;
        while bytes.len() > i {
            let attribute_flag = bytes[i];
            let attribute_length_octets = (attribute_flag & 0b00010000) + 1;
            let attribute_type_code = bytes[i + 1];
            let attribute_length = if attribute_length_octets == 1 {
                bytes[i + 2] as usize
            } else {
                u16::from_be_bytes(
                    bytes[i + 2..i + 4].try_into().context("aaa")?,
                ) as usize
            };

            let attribute_start_index =
                i + 1 + attribute_length_octets as usize + 1;
            let attribute_end_index = attribute_start_index + attribute_length;
            let path_attribute = match attribute_type_code {
                1 => PathAttribute::Origin(Origin::try_from(
                    bytes[attribute_start_index],
                )?),
                2 => PathAttribute::AsPath(AsPath::try_from(
                    &bytes[attribute_start_index..attribute_end_index],
                )?),
                3 => {
                    let addr = Ipv4Addr::new(
                        bytes[attribute_start_index],
                        bytes[attribute_start_index + 1],
                        bytes[attribute_start_index + 2],
                        bytes[attribute_start_index + 3],
                    );
                    PathAttribute::NextHop(addr)
                }
                _ => PathAttribute::DontKnow(
                    bytes[i..attribute_end_index].to_owned(),
                ),
            };
            path_attributes.push(path_attribute);
            i = attribute_end_index;
        }
        Ok(path_attributes)
    }
}

impl From<&PathAttribute> for BytesMut {
    fn from(p: &PathAttribute) -> BytesMut {
        let mut bytes = BytesMut::new();

        // PathAttributeのBytes表現は以下の通り
        // [Attribute Flag (1 octet)]
        // [Attribute Type Code(1 octet)]
        // [Attribute Length(1 or 2 octets)]
        // [Attribute毎の値 (Attribute Lengthのoctet数)]
        //
        // Attribute Flagは以下のBytes表現
        // - 1bit目: AttributeがOptionalなら1, Well-knownなら0
        // - 2bit目: Transitive（他ピアに伝える）なら1, そうじゃなければ0
        //           (補足: ただしWell-knownのものはすべてTransitive)
        // - 3bit目: Partialなら1, completeなら0。
        //           （Well-knownならすべてcomplete）
        // - 4bit目: Attribute Lengthがone octetなら0, two octetsなら1
        // - 5-8bit目: 使用しない。ゼロ
        match p {
            PathAttribute::Origin(o) => {
                let attribute_flag = 0b01000000;
                let attribute_type_code = 1;
                let attribute_length = 1;
                let attribute = match o {
                    Origin::Igp => 0,
                    Origin::Egp => 1,
                    Origin::Incomplete => 2,
                };
                bytes.put_u8(attribute_flag);
                bytes.put_u8(attribute_type_code);
                bytes.put_u8(attribute_length);
                bytes.put_u8(attribute);
            }
            PathAttribute::AsPath(a) => {
                let mut attribute_flag = 0b01000000;
                let attribute_type_code = 2;

                let attribute_length = a.bytes_len() as u16;
                let mut attribute_length_bytes = BytesMut::new();
                if attribute_length < 256 {
                    attribute_length_bytes.put_u8(attribute_length as u8);
                } else {
                    attribute_flag += 0b00010000;
                    attribute_length_bytes.put_u16(attribute_length);
                }

                let attribute = BytesMut::from(a);

                bytes.put_u8(attribute_flag);
                bytes.put_u8(attribute_type_code);
                bytes.put(attribute_length_bytes);
                bytes.put(attribute);
            }
            PathAttribute::NextHop(n) => {
                let mut attribute_flag = 0b01000000;
                let attribute_type_code = 3;
                let attribute_length = 4;
                let attribute = n.octets();

                bytes.put_u8(attribute_flag);
                bytes.put_u8(attribute_type_code);
                bytes.put_u8(attribute_length);
                bytes.put(&attribute[..]);
            }
            PathAttribute::DontKnow(v) => bytes.put(&v[..]),
        }
        bytes
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Origin {
    Igp,
    Egp,
    Incomplete,
}

impl TryFrom<u8> for Origin {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Origin::Igp),
            1 => Ok(Origin::Egp),
            2 => Ok(Origin::Incomplete),
            _ => Err(anyhow::anyhow!(format!(
                "value: {}をOriginに変換出来ませんでした。",
                value
            ))),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum AsPath {
    AsSequence(Vec<AutonomousSystemNumber>),
    AsSet(BTreeSet<AutonomousSystemNumber>),
}

impl From<&AsPath> for BytesMut {
    fn from(as_path: &AsPath) -> BytesMut {
        match as_path {
            AsPath::AsSet(s) => {
                let mut bytes = BytesMut::new();

                let path_segment_type = 1;
                let number_of_ases = s.len();
                bytes.put_u8(path_segment_type);
                bytes.put_u8(number_of_ases as u8);
                bytes.put(
                    &s.iter()
                        .flat_map(|a| u16::from(*a).to_be_bytes())
                        .collect::<Vec<u8>>()[..],
                );
                bytes
            }
            AsPath::AsSequence(s) => {
                let mut bytes = BytesMut::new();

                let path_segment_type = 2;
                let number_of_ases = s.len();
                bytes.put_u8(path_segment_type);
                bytes.put_u8(number_of_ases as u8);
                bytes.put(
                    &s.iter()
                        .flat_map(|a| u16::from(*a).to_be_bytes())
                        .collect::<Vec<u8>>()[..],
                );
                bytes
            }
        }
    }
}

impl AsPath {
    fn bytes_len(&self) -> usize {
        let as_bytes_length = match self {
            AsPath::AsSequence(v) => 2 * v.len(),
            AsPath::AsSet(s) => 2 * s.len(),
        };
        // AsSetかAsSequenceかを表すoctet + asの数を表すoctet + asのbytesの値
        1 + 1 + as_bytes_length
    }

    pub fn does_contain(&self, as_path: AutonomousSystemNumber) -> bool {
        match self {
            AsPath::AsSequence(seq) => seq.contains(&as_path),
            AsPath::AsSet(set) => set.contains(&as_path),
        }
    }

    pub fn push(&mut self, as_path: AutonomousSystemNumber) {
        match self {
            AsPath::AsSequence(seq) => seq.push(as_path),
            AsPath::AsSet(set) => {
                set.insert(as_path);
            }
        }
    }
}

impl TryFrom<&[u8]> for AsPath {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value[0] {
            1 => {
                let mut ases = BTreeSet::new();
                let mut i = 2;
                while i < value.len() {
                    ases.insert(
                        u16::from_be_bytes(value[i..i + 2].try_into()?).into(),
                    );
                    i += 2;
                }
                Ok(AsPath::AsSet(ases))
            }
            2 => {
                let mut ases = vec![];
                let mut i = 2;
                while i < value.len() {
                    ases.push(
                        u16::from_be_bytes(value[i..i + 2].try_into()?).into(),
                    );
                    i += 2;
                }
                Ok(AsPath::AsSequence(ases))
            }
            _ => Err(anyhow::anyhow!(format!(
                "value: {:?}をAsPathに変換出来ませんでした。",
                &value
            ))),
        }
    }
}

impl AsPath {
    pub fn add(&mut self, as_number: AutonomousSystemNumber) {
        match self {
            AsPath::AsSequence(seq) => seq.push(as_number),
            AsPath::AsSet(set) => {
                set.insert(as_number);
            }
        };
    }
}
