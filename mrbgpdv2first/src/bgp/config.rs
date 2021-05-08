use crate::bgp;
use std::{net::Ipv4Addr, str::FromStr};
pub struct Config {
    local_as_number: bgp::AutonomousSystemNumber,
    local_ip_address: Ipv4Addr,
    remote_as_number: bgp::AutonomousSystemNumber,
    remote_ip_address: Ipv4Addr,
    mode: Mode,
}

enum Mode {
    Passive,
    Active,
}

#[derive(Debug)]
struct ModeParseError;

impl FromStr for Mode {
    type Err = ModeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Mode::Active),
            "passive" => Ok(Mode::Passive),
            _ => Err(ModeParseError),
        }
    }
}

#[derive(Debug)]
pub struct ConfigParseError;

impl FromStr for Config {
    type Err = ConfigParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 64512 127.0.0.1 64513 127.0.0.2 activeのようなテキストがパース対象
        let c: Vec<&str> = s.split(" ").collect();
        let local_as_number =
            bgp::AutonomousSystemNumber(c[0].parse().expect("cannot parse local as number"));
        let local_ip_address = c[1].parse().expect("cannot parse local ip address");

        let remote_as_number =
            bgp::AutonomousSystemNumber(c[2].parse().expect("cannot parse local as number"));
        let remote_ip_address = c[3].parse().expect("cannot parse local ip address");

        let mode = c[4].parse().expect("cannot parse mode");

        Ok(Config {
            local_as_number,
            local_ip_address,
            remote_as_number,
            remote_ip_address,
            mode,
        })
    }
}
