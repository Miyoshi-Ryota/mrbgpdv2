use std::env;
use std::str::FromStr;

use mrbgpdv2::config::Config;
use mrbgpdv2::peer::Peer;

#[tokio::main]
async fn main() {
    let config = env::args().skip(1).fold("".to_owned(), |mut acc, s| {
        acc += &(s + " ");
        acc
    });
    let config = config.trim_end();
    let configs = vec![Config::from_str(config).unwrap()];
    let mut peers: Vec<Peer> = configs.into_iter().map(Peer::new).collect();
    for peer in &mut peers {
        peer.start();
    }
    for mut peer in peers {
        tokio::spawn( async move {
            loop {
                peer.next().await;
            }
        });
    }
}
