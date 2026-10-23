//! **誰が何を動かしたか**（UI 案・2 枚目「転送」／実運用の報告・2026-10-23）。
//!
//! ## なぜ要るか
//!
//! 帯には 1 行の文字列として出ていました。**構造として持っていなかった**ので、
//! 画面に「転送の列」を出せませんでした ——
//! 向き・誰が・何を・何バイト・成否を、並べて読む形が無かった。
//!
//! そして**柵の外へ書こうとして断られたこと**は、
//! 帯の 1 行に埋もれていました。案はそこを赤で「**止めました**」と出します。
//!
//! ## 何を持たないか
//!
//! **中身を持ちません。**動かしたファイルの名前と大きさだけです。
//! 中身を控えると、巻き戻し（D40）にも残ります。
//!
//! **秘密も持ちません。**パスフレーズやパスワードは、そもそもここへ来ません。
//!
//! **時刻も持ちません。**並びは `seq` で決まります ——
//! 同じミリ秒に 2 件来ても順が決まるように。

use std::collections::VecDeque;

use sshboard_band::Actor;

/// 上げたのか落としたのか。
/// **serde を付けません。**`sshboard-band` の `Actor` と同じ立場で、
/// **画面へ渡す形は画面側（`session_cmd.rs`）が決めます** ——
/// ここに付けると、エンジンが画面の都合を知ることになります。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 手元 → サーバー。
    Up,
    /// サーバー → 手元。**AI にこの口はありません**（D27）。
    Down,
}

/// どうなったか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// 動いた。
    Done,
    /// **止めた。**理由を持ちます ——「止めました」だけでは、
    /// 人は次に何をすればいいか分かりません。
    Refused { why: String },
}

/// 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    /// 並び順。**時刻ではありません** —— 同じミリ秒に 2 件来ても順が決まるように。
    pub seq: u64,
    /// 人か AI か。**ここが列の値打ち**です（案も行ごとに出しています）。
    pub actor: Actor,
    pub direction: Direction,
    /// 動かしたもの。**サーバー側の道が入るので、画面では伏せられる印を付けます**（禁止事項 4）。
    pub what: String,
    /// 何バイト。**止めたときは 0。**
    pub bytes: u64,
    pub outcome: Outcome,
}

/// **画面に残す件数の上限。**
///
/// `tail -f` のような流し込みは無いので帯より小さくしてありますが、
/// **青天井にはしません** —— 1 日動かし続けた機械で、
/// 画面が重くなる所を作らないため。
pub const MAX_KEPT: usize = 200;

/// 控え。**古い方から捨てます。**
///
/// 新しい方を捨てると、**いま起きていることが見えません。**
#[derive(Debug, Default)]
pub struct Transfers {
    held: VecDeque<Transfer>,
    next_seq: u64,
}

impl Transfers {
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 件足して、足したものを返す（画面へそのまま押し出せるように）。
    pub fn record(
        &mut self,
        actor: Actor,
        direction: Direction,
        what: impl Into<String>,
        bytes: u64,
        outcome: Outcome,
    ) -> Transfer {
        self.next_seq += 1;
        let line = Transfer {
            seq: self.next_seq,
            actor,
            direction,
            what: what.into(),
            // **止めたのに大きさが載っていると、動いたように読めます。**
            bytes: match outcome {
                Outcome::Done => bytes,
                Outcome::Refused { .. } => 0,
            },
            outcome,
        };
        self.held.push_back(line.clone());
        while self.held.len() > MAX_KEPT {
            self.held.pop_front();
        }
        line
    }

    /// 新しいものが後ろ。**並びは足した順そのまま。**
    pub fn all(&self) -> Vec<Transfer> {
        self.held.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_never_repeat() {
        // **同じ番号が 2 つ在ると、画面の並びが入れ替わります。**
        let mut held = Transfers::new();
        let a = held.record(Actor::Human, Direction::Up, "a", 1, Outcome::Done);
        let b = held.record(Actor::Ai, Direction::Up, "b", 2, Outcome::Done);
        assert_ne!(a.seq, b.seq);
        assert!(b.seq > a.seq);
    }

    #[test]
    fn a_refusal_carries_no_size() {
        // **止めたのに大きさが載っていると、動いたように読めます。**
        let mut held = Transfers::new();
        let line = held.record(
            Actor::Ai,
            Direction::Up,
            "/etc/nginx/conf.d/app.conf",
            4096,
            Outcome::Refused {
                why: "柵の外です".into(),
            },
        );
        assert_eq!(line.bytes, 0);
    }

    #[test]
    fn a_refusal_says_why() {
        // 「止めました」だけでは、**人は次に何をすればいいか分かりません。**
        let mut held = Transfers::new();
        let line = held.record(
            Actor::Ai,
            Direction::Up,
            "x",
            0,
            Outcome::Refused {
                why: "柵の外です".into(),
            },
        );
        match line.outcome {
            Outcome::Refused { why } => assert!(!why.trim().is_empty()),
            Outcome::Done => panic!("断りになっていない"),
        }
    }

    #[test]
    fn it_drops_the_oldest_not_the_newest() {
        // **新しい方を捨てると、いま起きていることが見えません。**
        let mut held = Transfers::new();
        for n in 0..MAX_KEPT + 3 {
            held.record(
                Actor::Human,
                Direction::Up,
                format!("f{n}"),
                1,
                Outcome::Done,
            );
        }
        let all = held.all();
        assert_eq!(all.len(), MAX_KEPT);
        assert_eq!(all.last().expect("空").what, format!("f{}", MAX_KEPT + 2));
        assert_eq!(all.first().expect("空").what, "f3");
    }

    #[test]
    fn it_keeps_who_moved_it() {
        // **ここが列の値打ち。**誰が動かしたかが行ごとに残ること。
        let mut held = Transfers::new();
        held.record(Actor::Human, Direction::Up, "a", 1, Outcome::Done);
        held.record(Actor::Ai, Direction::Down, "b", 2, Outcome::Done);
        let all = held.all();
        assert_eq!(all[0].actor, Actor::Human);
        assert_eq!(all[1].actor, Actor::Ai);
        assert_eq!(all[1].direction, Direction::Down);
    }
}
