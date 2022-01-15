/// BGP Messageなど通信に使うデータ構造を定義するモジュールです。
/// ここに定義されているデータ構造をBGP peer間でやり取りします。
mod header;
pub mod keepalive;
pub mod message;
pub mod open;
pub mod update;
