use super::event_queue::EventQueue;
use crate::bgp::config::Config;

struct Peer {
    config: Config,
    event_queue: EventQueue,
    now_state: State,
}

impl Peer {
    pub fn new(config: Config) -> Self {
        let event_queue = EventQueue::new();
        let now_state = State::Idle;
        Self {
            config,
            event_queue,
            now_state,
        }
    }

    pub fn start(&mut self) {
        self.event_queue.enqueue(Event::ManualStart);
    }

    pub fn next_step(&mut self) {
        if let Some(event) = self.event_queue.dequeue() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match self.now_state {
            State::Idle => match event {
                Event::ManualStart => {
                    self.now_state = State::Connect;
                }
                _ => {}
            },
            _ => (),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Idle,
    Connect,
    OpenSent,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Event {
    ManualStart,
    TcpCrAcked,
    TcpConnectionConfirmed,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_can_transition_to_connect_start() {
        let config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut bgp_peer = Peer::new(config);
        bgp_peer.start();
        bgp_peer.next_step();
        assert_eq!(bgp_peer.now_state, State::Connect);
    }
}
