#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn peer_can_transition_to_connect_state() {
        let config: Config = "64512 127.0.0.1 65413 127.0.0.2 active".parse().unwrap();
        let peer = Peer::new(config);
        peer.start();
        peer.next().await;
        assert_eq!(peer.state, State::Connect);
    }
}
