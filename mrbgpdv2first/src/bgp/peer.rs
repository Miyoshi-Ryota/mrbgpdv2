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
