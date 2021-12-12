/// BGPの[RFC内 8.1 で定義されているEvent](https://datatracker.ietf.org/doc/html/rfc4271#section-8.1)を
/// 表す列挙型です。
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum Event {
    ManualStart,
}
