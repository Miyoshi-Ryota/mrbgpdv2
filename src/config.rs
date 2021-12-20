use crate::bgp_type::AutonomousSystemNumber;
use crate::error::ConfigParseError;
use anyhow::{Context, Result};
use std::net::Ipv4Addr;
use std::str::FromStr;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord)]
pub struct Config {
    local_as: AutonomousSystemNumber,
    pub local_ip: Ipv4Addr,
    remote_as: AutonomousSystemNumber,
    pub remote_ip: Ipv4Addr,
    pub mode: Mode,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum Mode {
    Passive,
    Active,
}

impl FromStr for Mode {
    type Err = ConfigParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "passive" | "Passive" => Ok(Mode::Passive),
            "active" | "Active" => Ok(Mode::Active),
            _ => Err(ConfigParseError::from(anyhow::anyhow!("cannot parse {s}"))),
        }
    }
}

impl FromStr for Config {
    type Err = ConfigParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: Vec<&str> = s.split(' ').collect();
        let local_as = AutonomousSystemNumber::from(config[0].parse::<u16>().context(format!(
            "cannot parse 1st part of config, `{0}`, as as-number and config is {1}",
            config[0], s
        ))?);
        let local_ip: Ipv4Addr = config[1].parse().context(format!(
            "cannot parse 2nd part of config, `{0}`, as as-number and config is {1}",
            config[1], s
        ))?;
        let remote_as = AutonomousSystemNumber::from(config[2].parse::<u16>().context(format!(
            "cannot parse 3rd part of config, `{0}`, as as-number and config is {1}",
            config[2], s
        ))?);
        let remote_ip: Ipv4Addr = config[3].parse().context(format!(
            "cannot parse 4th part of config, `{0}`, as as-number and config is {1}",
            config[3], s
        ))?;
        let mode: Mode = config[4].parse().context(format!(
            "cannot parse 5th part of config, `{0}`, as as-number and config is {1}",
            config[4], s
        ))?;
        Ok(Self {
            local_as,
            local_ip,
            remote_as,
            remote_ip,
            mode,
        })
    }
}
