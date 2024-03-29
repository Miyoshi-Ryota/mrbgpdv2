use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

use crate::config::{Config, Mode};
use crate::connection::Connection;
use crate::event::Event;
use crate::event_queue::EventQueue;
use crate::packets::keepalive;
use crate::packets::message::Message;
use crate::packets::update::UpdateMessage;
use crate::routing::{AdjRibIn, AdjRibOut, LocRib};
use crate::state::State;

/// BGPのRFCで示されている実装方針
/// (https://datatracker.ietf.org/doc/html/rfc4271#section-8)では、
/// 1つのPeerを1つのイベント駆動ステートマシンとして実装しています。
/// Peer構造体はRFC内で示されている実装方針に従ったイベント駆動ステートマシンです。
#[derive(Debug)]
pub struct Peer {
    state: State,
    event_queue: EventQueue,
    tcp_connection: Option<Connection>,
    config: Config,
    loc_rib: Arc<Mutex<LocRib>>,
    adj_rib_out: AdjRibOut,
    adj_rib_in: AdjRibIn,
}

impl Peer {
    pub fn new(config: Config, loc_rib: Arc<Mutex<LocRib>>) -> Self {
        let state = State::Idle;
        let event_queue = EventQueue::new();
        let adj_rib_out = AdjRibOut::new();
        let adj_rib_in = AdjRibIn::new();
        Self {
            state,
            event_queue,
            config,
            tcp_connection: None,
            loc_rib,
            adj_rib_out,
            adj_rib_in,
        }
    }

    #[instrument]
    pub fn start(&mut self) {
        info!("peer is started.");
        self.event_queue.enqueue(Event::ManualStart);
    }

    #[instrument]
    pub async fn next(&mut self) {
        if let Some(event) = self.event_queue.dequeue() {
            info!("event is occured, event={:?}.", event);
            self.handle_event(event).await;
        }

        if let Some(conn) = &mut self.tcp_connection {
            if let Some(message) = conn.get_message().await {
                info!("message is recieved, message={:?}.", message);
                self.handle_message(message);
            }
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Open(open) => {
                self.event_queue.enqueue(Event::BgpOpen(open))
            }
            Message::Keepalive(keepalive) => {
                self.event_queue.enqueue(Event::KeepAliveMsg(keepalive))
            }
            Message::Update(update) => {
                self.event_queue.enqueue(Event::UpdateMsg(update))
            }
        }
    }

    #[instrument]
    async fn handle_event(&mut self, event: Event) {
        match &self.state {
            State::Idle => match event {
                Event::ManualStart => {
                    self.tcp_connection =
                        Connection::connect(&self.config).await.ok();
                    if self.tcp_connection.is_some() {
                        self.event_queue
                            .enqueue(Event::TcpConnectionConfirmed);
                    } else {
                        panic!(
                            "TCP Connectionの確立が出来ませんでした。{:?}",
                            self.config
                        )
                    }
                    self.state = State::Connect;
                }
                _ => {}
            },
            State::Connect => match event {
                Event::TcpConnectionConfirmed => {
                    self.tcp_connection
                        .as_mut()
                        .expect("TCP Connectionが確立できていません。")
                        .send(Message::new_open(
                            self.config.local_as,
                            self.config.local_ip,
                        ))
                        .await;
                    self.state = State::OpenSent
                }
                _ => {}
            },
            State::OpenSent => match event {
                Event::BgpOpen(open) => {
                    self.tcp_connection
                        .as_mut()
                        .expect("TCP Connectionが確立できていません。")
                        .send(Message::new_keepalive())
                        .await;
                    self.state = State::OpenConfirm;
                }
                _ => {}
            },
            State::OpenConfirm => match event {
                Event::KeepAliveMsg(keepalive) => {
                    self.state = State::Established;
                    self.event_queue.enqueue(Event::Established);
                }
                _ => {}
            },
            State::Established => match event {
                Event::Established | Event::LocRibChanged => {
                    debug!(
                        "before install routes from loc_rib \
                         to adj_rib_out: {:?}.",
                        self.adj_rib_out
                    );
                    let loc_rib = self.loc_rib.lock().await;
                    self.adj_rib_out
                        .install_from_loc_rib(&loc_rib, &self.config);
                    debug!(
                        "after install routes from loc_rib \
                         to adj_rib_out: {:?}.",
                        self.adj_rib_out
                    );
                    if self.adj_rib_out.does_contain_new_route() {
                        debug!("adj_rib_out is updated.");
                        self.event_queue.enqueue(Event::AdjRibOutChanged);
                        self.adj_rib_out.update_to_all_unchanged();
                    }
                }
                Event::AdjRibOutChanged => {
                    let updates: Vec<UpdateMessage> =
                        self.adj_rib_out.create_update_messages(
                            self.config.local_ip,
                            self.config.local_as,
                        );
                    for update in updates {
                        self.tcp_connection
                            .as_mut()
                            .expect("TCP Connectionが確立できていません。")
                            .send(Message::Update(update))
                            .await;
                    }
                }
                Event::UpdateMsg(update) => {
                    debug!(
                        "before install routes in \
                         update message to adj_rib_in: {:?}.",
                        self.adj_rib_in
                    );
                    self.adj_rib_in.install_from_update(update, &self.config);
                    debug!(
                        "after install routes in update message \
                         to adj_rib_in: {:?}.",
                        self.adj_rib_in
                    );
                    if self.adj_rib_in.does_contain_new_route() {
                        debug!("adj_rib in is updated.");
                        self.event_queue.enqueue(Event::AdjRibInChanged);
                        self.adj_rib_in.update_to_all_unchanged();
                    }
                }
                Event::AdjRibInChanged => {
                    debug!(
                        "before install routes from adj_rib_in \
                         to loc_rib: {:?}.",
                        self.loc_rib.lock().await
                    );
                    self.loc_rib
                        .lock()
                        .await
                        .install_from_adj_rib_in(&self.adj_rib_in);
                    debug!(
                        "after install routes from adj_rib to loc_rib: {:?}.",
                        self.loc_rib.lock().await
                    );
                    if self.loc_rib.lock().await.does_contain_new_route() {
                        info!("loc_rib is updated.");
                        self.loc_rib
                            .lock()
                            .await
                            .write_to_kernel_routing_table()
                            .await;
                        self.event_queue.enqueue(Event::LocRibChanged);
                        self.loc_rib.lock().await.update_to_all_unchanged();
                    }
                }
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn peer_can_transition_to_connect_state() {
        let config: Config =
            "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let loc_rib =
            Arc::new(Mutex::new(LocRib::new(&config).await.unwrap()));
        let mut peer = Peer::new(config, Arc::clone(&loc_rib));
        peer.start();

        // 別スレッドでPeer構造体を実行しています。
        // これはネットワーク上で離れた別のマシンを模擬しています。
        tokio::spawn(async move {
            let remote_config =
                "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let remote_loc_rib = Arc::new(Mutex::new(
                LocRib::new(&remote_config).await.unwrap(),
            ));
            let mut remote_peer =
                Peer::new(remote_config, Arc::clone(&remote_loc_rib));
            remote_peer.start();
            remote_peer.next().await;
        });

        // 先にremote_peer側の処理が進むことを保証するためのwait
        tokio::time::sleep(Duration::from_secs(1)).await;
        peer.next().await;
        assert_eq!(peer.state, State::Connect);
    }

    #[tokio::test]
    async fn peer_can_transition_to_open_sent_state() {
        let config: Config =
            "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let loc_rib =
            Arc::new(Mutex::new(LocRib::new(&config).await.unwrap()));
        let mut peer = Peer::new(config, Arc::clone(&loc_rib));
        peer.start();

        // 別スレッドでPeer構造体を実行しています。
        // これはネットワーク上で離れた別のマシンを模擬しています。
        tokio::spawn(async move {
            let remote_config =
                "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let remote_loc_rib = Arc::new(Mutex::new(
                LocRib::new(&remote_config).await.unwrap(),
            ));
            let mut remote_peer =
                Peer::new(remote_config, Arc::clone(&remote_loc_rib));
            remote_peer.start();
            remote_peer.next().await;
            remote_peer.next().await;
        });

        // 先にremote_peer側の処理が進むことを保証するためのwait
        tokio::time::sleep(Duration::from_secs(1)).await;
        peer.next().await;
        peer.next().await;
        assert_eq!(peer.state, State::OpenSent);
    }

    #[tokio::test]
    async fn peer_can_transition_to_open_confirm_state() {
        let config: Config =
            "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let loc_rib =
            Arc::new(Mutex::new(LocRib::new(&config).await.unwrap()));
        let mut peer = Peer::new(config, Arc::clone(&loc_rib));
        peer.start();

        // 別スレッドでPeer構造体を実行しています。
        // これはネットワーク上で離れた別のマシンを模擬しています。
        tokio::spawn(async move {
            let remote_config =
                "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let remote_loc_rib = Arc::new(Mutex::new(
                LocRib::new(&remote_config).await.unwrap(),
            ));
            let mut remote_peer =
                Peer::new(remote_config, Arc::clone(&remote_loc_rib));
            remote_peer.start();
            let max_step = 50;
            for _ in 0..max_step {
                remote_peer.next().await;
                if remote_peer.state == State::OpenConfirm {
                    break;
                };
                tokio::time::sleep(Duration::from_secs_f32(0.1)).await;
            }
        });

        // 先にremote_peer側の処理が進むことを保証するためのwait
        tokio::time::sleep(Duration::from_secs(1)).await;
        let max_step = 50;
        for _ in 0..max_step {
            peer.next().await;
            if peer.state == State::OpenConfirm {
                break;
            };
            tokio::time::sleep(Duration::from_secs_f32(0.1)).await;
        }
        assert_eq!(peer.state, State::OpenConfirm);
    }

    #[tokio::test]
    async fn peer_can_transition_to_established_state() {
        let config: Config =
            "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let loc_rib =
            Arc::new(Mutex::new(LocRib::new(&config).await.unwrap()));
        let mut peer = Peer::new(config, Arc::clone(&loc_rib));
        peer.start();

        // 別スレッドでPeer構造体を実行しています。
        // これはネットワーク上で離れた別のマシンを模擬しています。
        tokio::spawn(async move {
            let remote_config =
                "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let remote_loc_rib = Arc::new(Mutex::new(
                LocRib::new(&remote_config).await.unwrap(),
            ));
            let mut remote_peer =
                Peer::new(remote_config, Arc::clone(&remote_loc_rib));
            remote_peer.start();
            let max_step = 50;
            for _ in 0..max_step {
                remote_peer.next().await;
                if remote_peer.state == State::Established {
                    break;
                };
                tokio::time::sleep(Duration::from_secs_f32(0.1)).await;
            }
        });

        // 先にremote_peer側の処理が進むことを保証するためのwait
        tokio::time::sleep(Duration::from_secs(1)).await;
        let max_step = 50;
        for _ in 0..max_step {
            peer.next().await;
            if peer.state == State::Established {
                break;
            };
            tokio::time::sleep(Duration::from_secs_f32(0.1)).await;
        }
        assert_eq!(peer.state, State::Established);
    }
}
