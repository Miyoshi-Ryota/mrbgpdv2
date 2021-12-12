use std::str::FromStr;

use mrbgpdv2::config::Config;
use mrbgpdv2::peer::Peer;

#[tokio::main]
async fn main() {
    let configs = vec![Config::from_str("64512 127.0.0.1 65413 127.0.0.2 active").unwrap()];
    let mut peers: Vec<Peer> = configs.into_iter().map(Peer::new).collect();
    for peer in &mut peers {
        peer.start();
    }

    loop {
        for peer in &mut peers {
            peer.next().await;
        }
    }
}
