use crate::config::Config;
use crate::event::Event;
use crate::event_queue::EventQueue;
use crate::state::State;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
struct Peer {
    state: State,
    event_queue: EventQueue,
    config: Config,
}

impl Peer {
    fn new(config: Config) -> Self {
        let state = State::Idle;
        let event_queue = EventQueue::new();
        Self { state, event_queue, config }
    }

    fn start(&mut self) {
        self.event_queue.enqueue(Event::ManualStart);
    }

    async fn next(&mut self) {
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

    #[tokio::test]
    async fn peer_can_transition_to_connect_state() {
        let config: Config = "64512 127.0.0.1 65413 127.0.0.1 active".parse().unwrap();
        let mut peer = Peer::new(config);
        peer.start();
        peer.next().await;
        assert_eq!(peer.state, State::Connect);
    }
}
