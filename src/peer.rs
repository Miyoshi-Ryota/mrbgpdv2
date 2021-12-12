use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};

use crate::config::{Config, Mode};
use crate::event::Event;
use crate::event_queue::EventQueue;
use crate::state::State;

/// [BGPのRFCで示されている実装方針](https://datatracker.ietf.org/doc/html/rfc4271#section-8)では、
/// 1つのPeerを1つのイベント駆動ステートマシンとして実装しています。
/// Peer構造体はRFC内で示されている実装方針に従ったイベント駆動ステートマシンです。
#[derive(Debug)]
pub struct Peer {
    state: State,
    event_queue: EventQueue,
    tcp_connection: Option<TcpStream>,
    config: Config,
}

impl Peer {
    pub fn new(config: Config) -> Self {
        let state = State::Idle;
        let event_queue = EventQueue::new();
        Self {
            state,
            event_queue,
            config,
            tcp_connection: None,
        }
    }

    pub fn start(&mut self) {
        self.event_queue.enqueue(Event::ManualStart);
    }

    pub async fn next(&mut self) {
        if let Some(event) = self.event_queue.dequeue() {
            self.handle_event(&event).await;
        }
    }

    async fn handle_event(&mut self, event: &Event) {
        match &self.state {
            State::Idle => match event {
                Event::ManualStart => {
                    self.tcp_connection = match self.config.mode {
                        Mode::Active => self.connect_to_remote_peer().await,
                        Mode::Passive => self.wait_connection_from_remote_peer().await,
                    }
                    .ok();
                    self.tcp_connection.as_ref().unwrap_or_else(|| panic!("TCP Connectionの確立が出来ませんでした。{:?}",
                    self.config));
                    self.state = State::Connect;
                }
                _ => {}
            },
            _ => {}
        }
    }

    async fn connect_to_remote_peer(&self) -> Result<TcpStream> {
        let bgp_port = 179;
        TcpStream::connect((self.config.remote_ip, bgp_port))
            .await
            .context(format!(
                "cannot connect to remote peer {0}:{1}",
                self.config.remote_ip, bgp_port
            ))
    }

    async fn wait_connection_from_remote_peer(&self) -> Result<TcpStream> {
        let bgp_port = 179;
        let listener = TcpListener::bind((self.config.local_ip, bgp_port))
            .await
            .context(format!(
                "{0}:{1}にbindすることが出来ませんでした。",
                self.config.local_ip, bgp_port
            ))?;
        Ok(listener
            .accept()
            .await
            .context(format!(
                "{0}:{1}にてリモートからのTCP Connectionの要求を完遂することが出来ませんでした。
                リモートからTCP Connectionの要求が来ていない可能性が高いです。",
                self.config.local_ip, bgp_port
            ))?
            .0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn peer_can_transition_to_connect_state() {
        let config: Config = "64512 127.0.0.1 65413 127.0.0.2 active".parse().unwrap();
        let mut peer = Peer::new(config);
        peer.start();

        tokio::spawn(async move {
            let remote_config = "64513 127.0.0.2 65412 127.0.0.1 passive".parse().unwrap();
            let mut remote_peer = Peer::new(remote_config);
            remote_peer.start();
            remote_peer.next().await;
        });

        // 先にremote_peer側の処理が進むことを保証するためのwait
        tokio::time::sleep(Duration::from_secs(1)).await;
        peer.next().await;
        assert_eq!(peer.state, State::Connect);
    }
}
