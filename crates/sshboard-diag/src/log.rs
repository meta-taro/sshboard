//! 記録を溜める場所。**有界**で、**記録が実行を止めない。**

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::event::{Event, Level, Stage};

/// 溜めておく件数。
///
/// **無限に溜めない。**8 時間動かしっぱなしの道具なので、
/// 溜め続けるとメモリが減り続けます。**古いものから捨てます。**
pub const DEFAULT_CAPACITY: usize = 500;

struct Inner {
    events: Mutex<std::collections::VecDeque<Event>>,
    started: Instant,
    next_seq: AtomicU64,
    capacity: usize,
    /// 捨てた件数。**黙って消さない。**
    dropped: AtomicU64,
}

/// 何が起きたかの記録。**複製しても同じ 1 つを指します。**
#[derive(Clone)]
pub struct Diagnostics {
    inner: Arc<Inner>,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                events: Mutex::new(std::collections::VecDeque::with_capacity(capacity.min(64))),
                started: Instant::now(),
                next_seq: AtomicU64::new(0),
                // 0 件だと 1 件も残らず、記録の意味が消える。**最低 1 件は持つ。**
                capacity: capacity.max(1),
                dropped: AtomicU64::new(0),
            }),
        }
    }

    /// 1 件記録する。**待ちません。**記録が実行を止めてはいけません。
    pub fn record(
        &self,
        level: Level,
        stage: Stage,
        connection: Option<&str>,
        message: impl Into<String>,
        hint: Option<&str>,
    ) {
        let event = Event {
            seq: self.inner.next_seq.fetch_add(1, Ordering::SeqCst),
            at_ms: self.inner.started.elapsed().as_millis() as u64,
            level,
            stage,
            connection: connection.map(str::to_owned),
            message: message.into(),
            hint: hint.map(str::to_owned),
        };

        // 錠が壊れていても記録は諦める。**記録のために本体を止めない。**
        let Ok(mut held) = self.inner.events.lock() else {
            return;
        };
        if held.len() == self.inner.capacity {
            held.pop_front();
            self.inner.dropped.fetch_add(1, Ordering::SeqCst);
        }
        held.push_back(event);
    }

    /// 起きたことの報告。
    pub fn info(&self, stage: Stage, connection: Option<&str>, message: impl Into<String>) {
        self.record(Level::Info, stage, connection, message, None);
    }

    /// 通ったが、気に留めてほしいこと。
    pub fn warn(&self, stage: Stage, connection: Option<&str>, message: impl Into<String>) {
        self.record(Level::Warn, stage, connection, message, None);
    }

    /// 進めなかったこと。**次に何をすればよいかを必ず添える。**
    pub fn error(
        &self,
        stage: Stage,
        connection: Option<&str>,
        message: impl Into<String>,
        hint: &str,
    ) {
        self.record(Level::Error, stage, connection, message, Some(hint));
    }

    /// 新しい順に最大 `limit` 件。**AI へ返すのはここ。**
    pub fn recent(&self, limit: usize) -> Vec<Event> {
        let Ok(held) = self.inner.events.lock() else {
            return Vec::new();
        };
        held.iter().rev().take(limit).cloned().collect()
    }

    /// 溜まっている件数。
    pub fn len(&self) -> usize {
        self.inner.events.lock().map(|held| held.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 溢れて捨てた件数。**「全部見えている」と誤解させない。**
    pub fn dropped(&self) -> u64 {
        self.inner.dropped.load(Ordering::SeqCst)
    }
}
