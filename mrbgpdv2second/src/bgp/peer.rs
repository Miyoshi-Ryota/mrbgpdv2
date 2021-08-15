use super::{
    message::{BgpKeepaliveMessage, BgpOpenMessage},
    queue::{EventQueue, MessageQueue},
};
use crate::bgp::config::Config;
use crate::bgp::config::Mode;
use crate::bgp::message::{BgpMessage, BgpMessageHeader, BgpMessageType};
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
};

pub struct Peer {
    config: Config,
    event_queue: EventQueue,
    message_queue: MessageQueue,
    now_state: State,
    tcp_connection: Option<TcpStream>,
    buffer: Vec<u8>,
}

impl Peer {
    pub fn new(config: Config) -> Self {
        let event_queue = EventQueue::new();
        let message_queue = MessageQueue::new();
        let now_state = State::Idle;
        let tcp_connection = None;
        let buffer = vec![];
        Self {
            config,
            event_queue,
            message_queue,
            now_state,
            tcp_connection,
            buffer,
        }
    }

    pub fn start(&mut self) {
        self.event_queue.enqueue(Event::ManualStart);
    }

    pub fn next_step(&mut self) {
        if let Some(bgp_message) = self.recieve_one_message() {
            info!("Recive bgp message {:?}", bgp_message);
            self.handle_bgp_message(bgp_message)
        }

        if let Some(event) = self.event_queue.dequeue() {
            debug!("Now state {:?}, handling event {:?}", self.now_state, event);
            self.handle_event(event);
        }
    }

    fn transfer_data_tcp_connection_to_self_buffer(&mut self) {
        let mut buffer = vec![];
        if self.tcp_connection.is_some() {
            match self
                .tcp_connection
                .as_ref()
                .unwrap()
                .read_to_end(&mut buffer)
            {
                Ok(_) => (), // Tcp ConnectionがCloseしているときにOk()が返ってくる。
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.buffer.append(&mut buffer)
                }
                Err(e) => error!("{:?}", e),
            }
        }
    }

    fn handle_bgp_message(&mut self, bgp_message: BgpMessage) {
        match bgp_message.get_type() {
            BgpMessageType::Open => {
                self.event_queue.enqueue(Event::BgpOpen);
            }
            BgpMessageType::Keepalive => {
                self.event_queue.enqueue(Event::Keepalive);
            }
        }
        self.message_queue.enqueue(bgp_message);
    }

    fn retrive_one_message_from_buffer(&mut self) -> Option<Vec<u8>> {
        let minimum_length_of_bgp_message = 19;
        if self.buffer.len() >= minimum_length_of_bgp_message {
            let bgp_message_header = BgpMessageHeader::deserialize(&self.buffer[0..19].to_vec());
            let bgp_message_length: u16 = bgp_message_header.length;
            let (bgp_message_bytes, buf) = self.buffer.split_at(bgp_message_length as usize);
            let bgp_message_bytes = bgp_message_bytes.to_vec();
            self.buffer = buf.to_vec();
            Some(bgp_message_bytes)
        } else {
            None
        }
    }

    fn recieve_one_message(&mut self) -> Option<BgpMessage> {
        self.transfer_data_tcp_connection_to_self_buffer();
        self.retrive_one_message_from_buffer()
            .map(|bgp_message_byte| BgpMessage::deserialize(&bgp_message_byte))
    }

    fn create_tcp_connection_to_remote_ip(&mut self) -> Option<TcpStream> {
        let remote_addr = self.config.remote_ip_address;
        let bgp_port = 179;
        if self.config.mode == Mode::Active {
            let tcp_connection = TcpStream::connect((remote_addr, bgp_port)).ok();
            if tcp_connection.is_some() {
                self.event_queue.enqueue(Event::TcpCrAcked);
                tcp_connection
                    .as_ref()
                    .unwrap()
                    .set_nonblocking(true)
                    .unwrap();
            };
            tcp_connection
        } else {
            let tcp_listener = TcpListener::bind((self.config.local_ip_address, bgp_port))
                .expect("port 179にbind出来ません。");
            let tcp_connection = tcp_listener.accept().map(|v| v.0).ok();
            if tcp_connection.is_some() {
                self.event_queue.enqueue(Event::TcpConnectionConfirmed);
                tcp_connection
                    .as_ref()
                    .unwrap()
                    .set_nonblocking(true)
                    .unwrap();
            };
            tcp_connection
        }
    }

    fn send_bgp_message_to_remote_peer(&self, bgp_message: BgpMessage) {
        self.tcp_connection
            .as_ref()
            .unwrap()
            .write_all(&bgp_message.serialize()[..])
            .expect("Failed send open message");
        info!("Send bgp message {:?}", bgp_message);
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
                    let open_message = BgpOpenMessage::new(
                        self.config.local_as_number,
                        self.config.local_ip_address,
                    );
                    self.send_bgp_message_to_remote_peer(BgpMessage::Open(open_message));
                    self.now_state = State::OpenSent;
                }
                _ => {}
            },
            State::OpenSent => match event {
                Event::BgpOpen => {
                    let keepalive_message = BgpKeepaliveMessage::new();
                    self.send_bgp_message_to_remote_peer(BgpMessage::Keepalive(keepalive_message));
                    self.now_state = State::OpenConfirm;
                }
                _ => {}
            },
            State::OpenConfirm => match event {
                Event::Keepalive => {
                    self.now_state = State::Established;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Idle,
    Connect,
    OpenSent,
    OpenConfirm,
    Established,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Event {
    ManualStart,
    TcpCrAcked,
    TcpConnectionConfirmed,
    BgpOpen,
    Keepalive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn peer_can_transition_to_connect() {
        init();
        let config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut bgp_peer = Peer::new(config);
        bgp_peer.start();
        bgp_peer.next_step();
        assert_eq!(bgp_peer.now_state, State::Connect);
    }

    #[test]
    fn peer_can_transition_to_open_sent() {
        init();
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

    #[test]
    fn peer_can_transition_to_open_confirm() {
        init();
        let _remote_bgp = thread::spawn(|| {
            let remote_config: Config = "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let mut remote_bgp_peer = Peer::new(remote_config);
            remote_bgp_peer.start();

            let max_steps = 50;
            for _ in 0..max_steps {
                remote_bgp_peer.next_step();
                thread::sleep(time::Duration::from_secs_f32(0.1));
                if remote_bgp_peer.now_state == State::OpenConfirm {
                    break;
                };
            }

            assert_eq!(remote_bgp_peer.now_state, State::OpenConfirm);
        });

        // 先にPassiveモード側の処理が進むことを保証する。
        thread::sleep(time::Duration::from_secs_f32(0.5));

        let local_config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut local_bgp_peer = Peer::new(local_config);

        local_bgp_peer.start();
        let max_steps = 50;
        for _ in 0..max_steps {
            local_bgp_peer.next_step();
            thread::sleep(time::Duration::from_secs_f32(0.1));
            if local_bgp_peer.now_state == State::OpenConfirm {
                break;
            };
        }

        assert_eq!(local_bgp_peer.now_state, State::OpenConfirm);
    }

    #[test]
    fn peer_can_transition_to_established() {
        init();
        let _remote_bgp = thread::spawn(|| {
            let remote_config: Config = "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
            let mut remote_bgp_peer = Peer::new(remote_config);
            remote_bgp_peer.start();

            let max_steps = 50;
            for _ in 0..max_steps {
                remote_bgp_peer.next_step();
                thread::sleep(time::Duration::from_secs_f32(0.1));
                if remote_bgp_peer.now_state == State::Established {
                    break;
                };
            }

            assert_eq!(remote_bgp_peer.now_state, State::Established);
        });

        // 先にPassiveモード側の処理が進むことを保証する。
        thread::sleep(time::Duration::from_secs_f32(0.5));

        let local_config: Config = "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        let mut local_bgp_peer = Peer::new(local_config);

        local_bgp_peer.start();
        let max_steps = 50;
        for _ in 0..max_steps {
            local_bgp_peer.next_step();
            thread::sleep(time::Duration::from_secs_f32(0.1));
            if local_bgp_peer.now_state == State::Established {
                break;
            };
        }

        assert_eq!(local_bgp_peer.now_state, State::Established);
    }
}
