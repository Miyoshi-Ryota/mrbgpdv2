#[macro_use]
extern crate log;
use mrbgpdv2second::bgp::config::Config;
use mrbgpdv2second::bgp::peer::Peer;
use std::{env, thread, time};

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let config: Config = args[1..].join(" ").parse().unwrap();
    debug!("{:?}", config);
    let mut p = Peer::new(config);
    p.start();
    loop {
        p.next_step();
        thread::sleep(time::Duration::from_secs_f32(0.1));
    }
}