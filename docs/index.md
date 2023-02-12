# 作って学ぶルーティングプロトコル　RustでBGPを実装 サポートページ

本ページは商業誌版　[作って学ぶルーティングプロトコル　RustでBGPを実装](https://nextpublishing.jp/book/15905.html)のサポートページです。正誤表を記載します。本ページに記載のページ番号は全て物理本・PDF版のもので、Kindle版などではページ番号が異なる可能性があります。

## 目次
* toc
{:toc}

## 正誤表

### src/peer.rsのテストコード内のLocal AS番号とRemote AS番号の対応の誤り

src/peer.rsのテストコード内の以下のテストケースにおいて、Remote AS番号が誤っておりました。

#### 対象のテストケース一覧

| 対象の章 | 対象ページ | 対象のテストケース名 |
----|---- |----
| 2.4 最初のテストの追加 | P.17 | peer_can_transition_to_connect_state |
| 2.7 TCP Connectionの作成 | P25 | peer_can_transition_to_connect_state |
| 4.1.1 テストの追加 | P.36 | peer_can_transition_to_open_sent_state
| 4.2.1 テストの追加 | P.52 | peer_can_transition_to_open_confirm_state
| 4.3 Established Stateに遷移する | P.63 | peer_can_transition_to_established_state

本書内のコードでは、以下のように、`config`でRemote ASとして`65413`を指定し、`remote_config`でLocal ASとして`64513`を指定しておりミスマッチが起きています。

同様に`remote_config`でRemote ASとして`65412`を指定し、`config`でLocal ASとして`64512`と指定しミスマッチが起きています。

```[Rust]
#[cfg(test)]
mod tests {
     #[tokio::test]
     async fn peer_can_transition_to_connect_state() {
         let config: Config =
             "64512 127.0.0.1 65413 127.0.0.2 active".parse().unwrap();
        <略>
         // これはネットワーク上で離れた別のマシンを模擬しています。
         tokio::spawn(async move {
             let remote_config =
                 "64513 127.0.0.2 65412 127.0.0.1 passive".parse().unwrap();
        <略>
    }
}
```

AS番号として`64512`、`64513`を指定することが正しい内容です。以下に修正版のコードを掲載します。

```[Rust]
#[cfg(test)]
mod tests {
     #[tokio::test]
     async fn peer_can_transition_to_connect_state() {
         let config: Config =
             "64512 127.0.0.1 64513 127.0.0.2 active".parse().unwrap();
        <略>
         // これはネットワーク上で離れた別のマシンを模擬しています。
         tokio::spawn(async move {
             let remote_config =
                 "64513 127.0.0.2 64512 127.0.0.1 passive".parse().unwrap();
        <略>
    }
}
```

差分がわかりづらい場合は、[サンプルコードのリポジトリの修正コミット](https://github.com/Miyoshi-Ryota/mrbgpdv2/commit/26bdddad468731e903045065ac68a31e8e01cd14
)を参照ください。


### 4.1.3章 「Open Messageを送信する。」のP.50でsrc/peer.rs内の`handle_event`メソッドに追加したOpenMessageの送信処理がそれ以降の4章内の掲載コード（P.56, P.62, P.65）では記載されていない

4.1.3章の「Open Messageを送信する。」のP.50で以下のようにOpen Messageの送信の処理を追加しました。

```Rust
+use crate::packets::message::Message;

     async fn handle_event(&mut self, event: Event) {
         match &self.state {
             State::Connect => match event {
                 Event::TcpConnectionConfirmed => {
+                    self.tcp_connection
+                        .as_mut()
+                        .expect("TCP Connectionが確立できていません。")
+                        .send(Message::new_open(
+                            self.config.local_as,
+                            self.config.local_ip,
+                        ))
+                        .await;
                     self.state = State::OpenSent
                 }
                 _ => {}
             },
```

P.56, P.62, P.65などのサンプルコードでは同様のメソッド内のmatch式の`State::Connect`の枝は以下のように記載されており、P.50で追加したOpen Messageの送信の処理が消えています。

```Rust
     async fn handle_event(&mut self, event: Event) {
         match &self.state {
             State::Idle => match event {
                 Event::ManualStart => {
                     self.tcp_connection =
                         Connection::connect(&self.config).await.ok();
                     if self.tcp_connection.is_some() {
                         self.event_queue
                             .enqueue(Event::TcpConnectionConfirmed);
                     } else {
                         panic!(
                             "TCP Connectionの確立が出来ませんでした。{:?}",
                             self.config
                         )
                     }
                     self.state = State::Connect;
                 }
                 _ => {}
             },
             State::Connect => match event {
                 Event::TcpConnectionConfirmed => {
                     self.state = State::OpenSent
                 }
                 _ => {}
             },
+            State::OpenSent => match event {
+                Event::BgpOpen(open) => {
+                    // ToDo: Keepalive messageを送信する。
+                    self.state = State::OpenConfirm;
+                }
+                _ => {}
+            },
         }
     }
```
