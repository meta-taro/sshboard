//! 帯そのもの。1 行載せて、全購読者が受け取りを返すまで待てる。
//!
//! **なぜ ack が要るか**（Issue 001 の完了条件）:
//! MCP ツールが応答を返したあとに画面が追いつく形にすると、
//! 「AI が先に動いて、人はあとから知る」ことになる。それは見えているとは言わない。
//! そこで、ツール応答を返す前に「帯へ届いたこと」を待てる口をここに置く。

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Notify};

use crate::line::{Actor, BandLine};

/// 購読者が詰まったときに落とすまでの猶予。
/// 帯は人が読むものなので、これを超えて溜まる時点で読めていない。
const CHANNEL_CAPACITY: usize = 1024;

/// 帯へ 1 行載せた結果。`wait_acked` で「届いたか」を待てる。
pub struct Delivery {
    line: BandLine,
    gate: Arc<AckGate>,
}

/// 配達の結末。**握り潰さない。**期限切れは呼び出し側が知る必要がある。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// 全購読者が受け取りを返した。
    Delivered,
    /// 期限内に返さなかった購読者がいる。
    TimedOut { acked: usize, expected: usize },
}

/// 購読者へ届く 1 通。**処理し終えたら `ack()` を呼ぶこと。**
#[derive(Debug, Clone)]
pub struct BandEvent {
    line: BandLine,
    gate: Arc<AckGate>,
}

impl BandEvent {
    pub fn line(&self) -> &BandLine {
        &self.line
    }

    /// この行を「画面へ出した」と帯へ返す。
    pub fn ack(&self) {
        self.gate.ack();
    }
}

/// 帯の購読口。
pub struct Subscriber {
    rx: broadcast::Receiver<BandEvent>,
}

impl Subscriber {
    pub async fn recv(&mut self) -> Result<BandEvent, broadcast::error::RecvError> {
        self.rx.recv().await
    }
}

struct Inner {
    tx: broadcast::Sender<BandEvent>,
    next_seq: AtomicU64,
}

/// 人と AI の操作が流れる 1 本の帯。
#[derive(Clone)]
pub struct Band {
    inner: Arc<Inner>,
}

impl Band {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                tx,
                next_seq: AtomicU64::new(0),
            }),
        }
    }

    pub fn subscribe(&self) -> Subscriber {
        Subscriber {
            rx: self.inner.tx.subscribe(),
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.inner.tx.receiver_count()
    }

    /// 帯へ 1 行載せる。**この時点で購読者への送出は済んでいる。**
    ///
    /// 待つ相手の数は送出の直前に数える。数えたあとに購読者が増減した場合、
    /// 増えた側は数に入らず（余分な ack は捨てる）、減った側は `wait_acked` が
    /// 期限切れになる。**画面を開閉した瞬間だけ起きる誤差で、行は落とさない。**
    pub fn record(&self, actor: Actor, text: impl Into<String>) -> Delivery {
        let seq = self.inner.next_seq.fetch_add(1, Ordering::SeqCst);
        let line = BandLine::new(seq, actor, text);
        let gate = Arc::new(AckGate::new(self.inner.tx.receiver_count()));

        // 購読者が 0 のときは Err が返るが、それは異常ではない（画面がまだ無い）。
        let _ = self.inner.tx.send(BandEvent {
            line: line.clone(),
            gate: Arc::clone(&gate),
        });

        Delivery { line, gate }
    }
}

impl Default for Band {
    fn default() -> Self {
        Self::new()
    }
}

impl Delivery {
    pub fn line(&self) -> &BandLine {
        &self.line
    }

    /// 全購読者が受け取りを返すまで待つ。購読者が 0 なら即座に `Delivered`。
    pub async fn wait_acked(&self, timeout: Duration) -> DeliveryOutcome {
        if tokio::time::timeout(timeout, self.gate.wait())
            .await
            .is_ok()
        {
            return DeliveryOutcome::Delivered;
        }
        self.gate.timed_out()
    }
}

/// ack を数える門。`remaining` が 0 になったら待っている者を起こす。
#[derive(Debug)]
struct AckGate {
    expected: usize,
    remaining: AtomicUsize,
    notify: Notify,
}

impl AckGate {
    fn new(expected: usize) -> Self {
        Self {
            expected,
            remaining: AtomicUsize::new(expected),
            notify: Notify::new(),
        }
    }

    /// 余分な ack で 0 を下回らせない。購読者が同じ通を 2 回 ack しても、
    /// ほかの購読者の分を勝手に消さないようにする。
    fn ack(&self) {
        let updated = self
            .remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            });

        if updated == Ok(1) {
            self.notify.notify_waiters();
        }
    }

    async fn wait(&self) {
        loop {
            // 先に登録してから数える。逆にすると、その隙間に来た通知を取り落とす。
            let notified = self.notify.notified();
            if self.remaining.load(Ordering::SeqCst) == 0 {
                return;
            }
            notified.await;
        }
    }

    fn timed_out(&self) -> DeliveryOutcome {
        let left = self.remaining.load(Ordering::SeqCst);
        DeliveryOutcome::TimedOut {
            acked: self.expected - left,
            expected: self.expected,
        }
    }
}
