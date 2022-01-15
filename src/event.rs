use crate::packets::{keepalive::KeepaliveMessage, open::OpenMessage, update::UpdateMessage};

/// BGPの[RFC内 8.1 で定義されているEvent](https://datatracker.ietf.org/doc/html/rfc4271#section-8.1)を
/// 表す列挙型です。
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum Event {
    ManualStart,
    // 正常系しか実装しない本実装では別のEventとして扱う意味がないため、
    // TcpConnectionConfirmedはTcpCrAckedも兼ねている。
    TcpConnectionConfirmed,
    BgpOpen(OpenMessage),
    KeepAliveMsg(KeepaliveMessage), // MsgはMessageの省略形。BGPのRFC内での定義に従っている。
    UpdateMsg(UpdateMessage),       // BGPのRFC内での定義に従っている。
    Established, // StateがEstablishedに遷移したことを表す。存在するほうが実装が楽なので追加した本実装オリジナルのイベント
    // LocRib / AdjRibOutが変わったときのイベント。存在するほうが実装が楽なので追加した。
    LocRibChanged,
    AdjRibOutChanged,
}
