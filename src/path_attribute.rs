use crate::bgp_type::AutonomousSystemNumber;
use std::{collections::HashSet, net::Ipv4Addr};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PathAttribute {
    Origin(Origin),
    AsPath(AsPath),
    NextHop(Ipv4Addr),
    DontKnow(Vec<u8>), // 対応してないPathAttribute用
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Origin {
    Igp,
    Egp,
    Incomplete,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AsPath {
    AsSequence(Vec<AutonomousSystemNumber>),
    AsSet(HashSet<AutonomousSystemNumber>),
}
