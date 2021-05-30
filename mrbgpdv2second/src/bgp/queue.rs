use crate::bgp::message::BgpMessage;
use crate::bgp::peer::Event;
use std::collections::VecDeque;

pub struct Queue<T>(VecDeque<T>);

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue(VecDeque::new())
    }

    pub fn enqueue(&mut self, data: T) {
        self.0.push_front(data);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.0.pop_back()
    }
}

pub type EventQueue = Queue<Event>;
pub type MessageQueue = Queue<BgpMessage>;
