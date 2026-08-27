//! 接続一覧が変わったことを知らせる。
//!
//! **なぜ要るか**: 一覧は人（GUI）と AI（MCP）の両方が書き換える。
//! **画面が開いたときに 1 回読むだけだと、AI が足した接続を人が知らないまま**になる。
//! それは PRD §4-2「AI の操作が人の画面にその場で流れる」に反する。
//!
//! **中身は流しません。**「変わった」ことだけを流し、読み直しは受け取った側がやります。
//! 中身を流すと、購読者ごとに接続先が配られることになる（PRD §8）。

use tokio::sync::broadcast;

/// 溜まっても意味が無い通知なので、控えめでよい。
const CHANNEL_CAPACITY: usize = 16;

/// 「接続一覧が変わった」を配る口。
pub struct ConnectionsWatch {
    tx: broadcast::Sender<()>,
}

impl ConnectionsWatch {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self { tx }
    }

    /// 書き換えたあとに呼ぶ。**購読者が 0 でも失敗ではない。**
    pub fn notify(&self) {
        let _ = self.tx.send(());
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }
}

impl Default for ConnectionsWatch {
    fn default() -> Self {
        Self::new()
    }
}
