//! 1 本の出力を、面ごとに違う形で配る。
//!
//! **同じ出力を 2 回実行しない。**`tail -f` を 2 本流すのは
//! 「裏で見えないセッションを張らない」（PRD §4-1）に反する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::plain::PlainFilter;

/// 追いつかない購読者を切るまでの猶予。
const CHANNEL_CAPACITY: usize = 1024;

/// 止まっているのに流そうとした。
#[derive(Debug, PartialEq, Eq)]
pub struct StreamStopped;

impl std::fmt::Display for StreamStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "この出力は止められています")
    }
}

impl std::error::Error for StreamStopped {}

/// 生（GUI 向け）と素（MCP 向け）を、1 回の `push` で同時に配る。
pub struct OutputStream {
    raw: broadcast::Sender<Vec<u8>>,
    plain: broadcast::Sender<String>,
    filter: Mutex<PlainFilter>,
    stopped: AtomicBool,
}

impl OutputStream {
    pub fn new() -> Self {
        let (raw, _) = broadcast::channel(CHANNEL_CAPACITY);
        let (plain, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            raw,
            plain,
            filter: Mutex::new(PlainFilter::new()),
            stopped: AtomicBool::new(false),
        }
    }

    /// GUI 向け。**ANSI を落とさない。**
    pub fn subscribe_raw(&self) -> broadcast::Receiver<Vec<u8>> {
        self.raw.subscribe()
    }

    /// MCP 向け。**ANSI が 1 つも混ざらない。**
    pub fn subscribe_plain(&self) -> broadcast::Receiver<String> {
        self.plain.subscribe()
    }

    /// チャンクを 1 つ流す。**両方の面へ同時に出る。**
    ///
    /// 購読者が 0 でも失敗にしない。画面を閉じていても MCP 側は流れ続ける（逆も同じ）。
    pub fn push(&self, chunk: &[u8]) -> Result<(), StreamStopped> {
        if self.is_stopped() {
            return Err(StreamStopped);
        }

        // 生を先に出す。人が見ている面を待たせない。
        let _ = self.raw.send(chunk.to_vec());

        // フィルタが panic することは無いが、仮に毒されても止めない。
        // ここで諦めると、以後 MCP 側だけが黙って死ぬ。
        let mut filter = self
            .filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let text = filter.push(chunk);
        drop(filter);

        // 空文字を送らない。エスケープだけのチャンクで購読者を起こさない。
        if !text.is_empty() {
            let _ = self.plain.send(text);
        }

        Ok(())
    }

    /// 人が止める（PRD §4-3）。**以後の `push` は拒否される。**
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

impl Default for OutputStream {
    fn default() -> Self {
        Self::new()
    }
}
