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
