pub mod config;
mod message;
pub mod peer;
mod queue;
mod timer;

#[derive(Debug, Copy, Clone)]
pub struct AutonomousSystemNumber(u16);
