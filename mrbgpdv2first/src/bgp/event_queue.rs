use crate::bgp::peer::Event;
use std::collections::VecDeque;

pub struct EventQueue(VecDeque<Event>);

impl EventQueue {
    pub fn new() -> Self {
        EventQueue(VecDeque::new())
    }

    pub fn enqueue(&mut self, event: Event) {
        self.0.push_front(event);
    }

    pub fn dequeue(&mut self) -> Option<Event> {
        self.0.pop_back()
    }
}
