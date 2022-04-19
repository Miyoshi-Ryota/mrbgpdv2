use std::env;
use std::str::FromStr;
use std::sync::Arc;

use mrbgpdv2::config::Config;
use mrbgpdv2::peer::Peer;
use mrbgpdv2::routing::LocRib;
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() {
    let config = env::args().skip(1).fold("".to_owned(), |mut acc, s| {
        acc += &(s + " ");
        acc
    });
    let config = config.trim_end();
    let configs =
        vec![Config::from_str(config).expect("引数からConfig構造体の作成に失敗しました。")];

    tracing_subscriber::fmt::init();
    info!("mrbgpdv2 started with configs {:?}.", configs);

    // ToDo: configs[0]ではなく、アドバタイズするnetworkのvecを引数に取るようにする。
    // Configはpeerごとなのに、loc_ribはすべてのpeerで共有する。Peer毎のコンフィグから
    // 共有するものを生成することに違和感があるため。
    let loc_rib = Arc::new(Mutex::new(
        LocRib::new(&configs[0])
            .await
            .expect("LocRibの生成に失敗しました。"),
    ));
    let mut peers: Vec<Peer> = configs
        .into_iter()
        .map(|c| Peer::new(c, Arc::clone(&loc_rib)))
        .collect();
    for peer in &mut peers {
        peer.start();
    }
    let mut handles = vec![];
    for mut peer in peers {
        let handle = tokio::spawn(async move {
            loop {
                peer.next().await;
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.await;
    }
}
