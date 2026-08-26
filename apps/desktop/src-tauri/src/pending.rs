//! 画面へ出したが、まだ「出した」と返ってきていない行を預かる場所。
//!
//! **ここに残っている＝まだ人の目に入っていない。**
//! だから、預かったまま勝手に ack しない。ack は画面が描いたときだけ返す（D16）。

use std::collections::BTreeMap;
use std::sync::Mutex;

use sshboard_band::BandEvent;

/// ack 待ちで保持する上限。画面が受け取らないまま溜まり続けるのを止める。
const MAX_PENDING: usize = 512;

/// 預かりに失敗した理由。**握り潰さない。**
#[derive(Debug, PartialEq, Eq)]
pub enum PendingError {
    /// 別のスレッドが保持中にパニックした。
    Poisoned,
    /// 画面が知らない行を ack してきた。
    Unknown { seq: u64 },
}

impl std::fmt::Display for PendingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PendingError::Poisoned => write!(f, "band buffer is poisoned"),
            PendingError::Unknown { seq } => write!(f, "no line is waiting for seq {seq}"),
        }
    }
}

/// ack 待ちの行。
pub struct PendingLines {
    lines: Mutex<BTreeMap<u64, BandEvent>>,
}

impl PendingLines {
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(BTreeMap::new()),
        }
    }

    /// 画面へ出す直前に預ける。
    ///
    /// 上限を超えた分は古い方から落とす。**落とすときに ack しない。**
    /// 出していないものを「出した」と返すと、帯の意味が無くなる。
    pub fn hold(&self, event: BandEvent) -> Result<(), PendingError> {
        let mut lines = self.lines.lock().map_err(|_| PendingError::Poisoned)?;
        lines.insert(event.line().seq(), event);

        while lines.len() > MAX_PENDING {
            let oldest = *lines
                .keys()
                .next()
                .expect("空でないことは len で確かめている");
            lines.remove(&oldest);
        }
        Ok(())
    }

    /// 画面が「描いた」と言ってきた行を ack する。
    pub fn release(&self, seq: u64) -> Result<(), PendingError> {
        let event = {
            let mut lines = self.lines.lock().map_err(|_| PendingError::Poisoned)?;
            lines.remove(&seq)
        };

        match event {
            Some(event) => {
                event.ack();
                Ok(())
            }
            None => Err(PendingError::Unknown { seq }),
        }
    }

    /// いまのところテストからしか見ない。**本体から使い始めたらこの gate を外すこと。**
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.lines.lock().map(|lines| lines.len()).unwrap_or(0)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PendingLines {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sshboard_band::{Actor, Band, DeliveryOutcome};

    use super::*;

    #[tokio::test]
    async fn releasing_a_held_line_acks_it_on_the_band() {
        // Arrange
        let band = Band::new();
        let mut subscriber = band.subscribe();
        let pending = PendingLines::new();
        let delivery = band.record(Actor::Ai, "ping");
        let event = subscriber.recv().await.expect("帯へ出ていない");

        // Act
        pending.hold(event).expect("預けられない");
        let before = delivery.wait_acked(Duration::from_millis(50)).await;
        pending.release(0).expect("ack できない");
        let after = delivery.wait_acked(Duration::from_millis(500)).await;

        // Assert
        assert_eq!(
            before,
            DeliveryOutcome::TimedOut {
                acked: 0,
                expected: 1
            }
        );
        assert_eq!(after, DeliveryOutcome::Delivered);
        assert!(pending.is_empty(), "ack 済みの行が残っている");
    }

    #[tokio::test]
    async fn releasing_a_line_nobody_holds_is_an_error_not_a_silent_no_op() {
        // Arrange
        let pending = PendingLines::new();

        // Act
        let result = pending.release(7);

        // Assert
        assert_eq!(result, Err(PendingError::Unknown { seq: 7 }));
    }

    #[tokio::test]
    async fn a_dropped_overflow_line_is_never_acked() {
        // 出していない行を「出した」と返さないこと。
        // Arrange
        let band = Band::new();
        let mut subscriber = band.subscribe();
        let pending = PendingLines::new();
        let first = band.record(Actor::Ai, "oldest");
        pending
            .hold(subscriber.recv().await.expect("帯へ出ていない"))
            .expect("預けられない");

        // Act — 上限を超えるまで詰める
        for _ in 0..MAX_PENDING {
            band.record(Actor::Ai, "filler");
            pending
                .hold(subscriber.recv().await.expect("帯へ出ていない"))
                .expect("預けられない");
        }

        // Assert
        assert_eq!(pending.len(), MAX_PENDING, "上限を超えて保持している");
        assert_eq!(
            first.wait_acked(Duration::from_millis(50)).await,
            DeliveryOutcome::TimedOut {
                acked: 0,
                expected: 1
            },
            "落とした行を ack してしまっている"
        );
        assert_eq!(pending.release(0), Err(PendingError::Unknown { seq: 0 }));
    }
}
