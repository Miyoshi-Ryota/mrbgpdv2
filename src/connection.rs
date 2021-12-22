use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config::{Config, Mode};
use crate::error::CreateConnectionError;
use crate::packets::message::Message;

/// 通信に関する処理を担当する構造体です。
/// TcpConnectionを張ったり、crate::packets::message::Messageのデータを送受信したりします。
#[derive(Debug)]
pub struct Connection {
    conn: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub async fn connect(config: &Config) -> Result<Self, CreateConnectionError> {
        let conn = match config.mode {
            Mode::Active => Self::connect_to_remote_peer(config).await,
            Mode::Passive => Self::wait_connection_from_remote_peer(config).await,
        }?;
        let buffer = BytesMut::with_capacity(1500);
        Ok(Self { conn, buffer })
    }

    pub async fn send(&mut self, message: Message) {
        let bytes: BytesMut = message.into();
        self.conn.write(&bytes[..]).await;
    }

    async fn connect_to_remote_peer(config: &Config) -> Result<TcpStream> {
        let bgp_port = 179;
        TcpStream::connect((config.remote_ip, bgp_port))
            .await
            .context(format!(
                "cannot connect to remote peer {0}:{1}",
                config.remote_ip, bgp_port
            ))
    }

    async fn wait_connection_from_remote_peer(config: &Config) -> Result<TcpStream> {
        let bgp_port = 179;
        let listener = TcpListener::bind((config.local_ip, bgp_port))
            .await
            .context(format!(
                "{0}:{1}にbindすることが出来ませんでした。",
                config.local_ip, bgp_port
            ))?;
        Ok(listener
            .accept()
            .await
            .context(format!(
                "{0}:{1}にてリモートからのTCP Connectionの要求を完遂することが出来ませんでした。
                リモートからTCP Connectionの要求が来ていない可能性が高いです。",
                config.local_ip, bgp_port
            ))?
            .0)
    }
}
