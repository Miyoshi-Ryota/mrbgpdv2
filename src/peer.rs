use crate::config::Config;
use crate::event::Event;
use crate::event_queue::EventQueue;
use crate::state::State;

/// [BGPのRFCで示されている実装方針](https://datatracker.ietf.org/doc/html/rfc4271#section-8)では、
/// 1つのPeerを1つのイベント駆動ステートマシンとして実装しています。
/// Peer構造体はRFC内で示されている実装方針に従ったイベント駆動ステートマシンです。
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct Peer {
    state: State,
    event_queue: EventQueue,
    config: Config,
}

impl Peer {
    pub fn new(config: Config) -> Self {
        let state = State::Idle;
        let event_queue = EventQueue::new();
        Self { state, event_queue, config }
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
            State::Idle => {
                match event {
                    Event::ManualStart => {
                        self.state = State::Connect;
                    }
                    _ => {}
                }
            },
            _ => {}
        }
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
