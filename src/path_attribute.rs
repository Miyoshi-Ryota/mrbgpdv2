use bytes::{BufMut, BytesMut};

use crate::bgp_type::AutonomousSystemNumber;
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
        match self {
            PathAttribute::Origin(o) => 1,
            PathAttribute::AsPath(a) => a.bytes_len(),
            PathAttribute::NextHop(_) => 4,
            PathAttribute::DontKnow(v) => v.len(),
        }
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
                let attribute_type_code = 2;
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
                        .map(|a| u16::from(*a).to_be_bytes())
                        .flatten()
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
                        .map(|a| u16::from(*a).to_be_bytes())
                        .flatten()
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
