use super::event_queue::EventQueue;
use crate::bgp::config::Config;
use crate::bgp::config::Mode;
use std::net::{TcpListener, TcpStream};

struct Peer {
    config: Config,
    event_queue: EventQueue,
    now_state: State,
    tcp_connection: Option<TcpStream>,
}

impl Peer {
    pub fn new(config: Config) -> Self {
        let event_queue = EventQueue::new();
        let now_state = State::Idle;
        let tcp_connection = None;
        Self {
            config,
            event_queue,
            now_state,
            tcp_connection,
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

    fn create_tcp_connection_to_remote_ip(&mut self) -> Option<TcpStream> {
        let remote_addr = self.config.remote_ip_address;
        let bgp_port = 179;
        if self.config.mode == Mode::Active {
            let tcp_connection = TcpStream::connect((remote_addr, bgp_port)).ok();
            if tcp_connection.is_some() {
                self.event_queue.enqueue(Event::TcpCrAcked);
            };
            tcp_connection
        } else {
            let tcp_listener = TcpListener::bind((self.config.local_ip_address, bgp_port)).expect("port 179にbind出来ません。");
            let tcp_connection = tcp_listener.accept().map(|v| v.0).ok();
            if tcp_connection.is_some() {
                self.event_queue.enqueue(Event::TcpConnectionConfirmed);
            };
            tcp_connection
        }
    }

    fn handle_event(&mut self, event: Event) {
        match self.now_state {
            State::Idle => match event {
                Event::ManualStart => {
                    self.tcp_connection = self.create_tcp_connection_to_remote_ip();
                    self.now_state = State::Connect;
                }
                _ => {}
            },
            State::Connect => match event {
                Event::TcpConnectionConfirmed | Event::TcpCrAcked => {
                    self.now_state = State::OpenSent;
                },
                _ => {},
            },
            _ => {},
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
    use std::thread;
    use std::time;

    #[test]
    fn peer_can_transition_to_connect_start() {
        let config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut bgp_peer = Peer::new(config);
        bgp_peer.start();
        bgp_peer.next_step();
        assert_eq!(bgp_peer.now_state, State::Connect);
    }

    #[test]
    fn peer_can_transition_to_open_sent_start() {
        let _remote_bgp = thread::spawn(|| {
            let remote_config: Config = "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let mut remote_bgp_peer = Peer::new(remote_config);
            remote_bgp_peer.start();
            remote_bgp_peer.next_step();
            remote_bgp_peer.next_step();
            assert_eq!(remote_bgp_peer.now_state, State::OpenSent);
        });

        // 先にPassiveモード側の処理が進むことを保証する。
        thread::sleep(time::Duration::from_secs_f32(0.5));

        let local_config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut local_bgp_peer = Peer::new(local_config);

        local_bgp_peer.start();
        local_bgp_peer.next_step();
        local_bgp_peer.next_step();

        assert_eq!(local_bgp_peer.now_state, State::OpenSent);
    }
}
