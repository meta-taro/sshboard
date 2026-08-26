//! 帯に流れる 1 行。
//!
//! PRD §4-2「面が違っても記録は 1 本」。ファイル面の操作も端末の操作も、
//! MCP からの呼び出しも、すべてこの型になってから帯へ載る。

/// その操作を誰がやったか。
///
/// **この 2 つしかない。**「システム」や「不明」を足さないこと。
/// 誰がやったか分からない行が帯に出た時点で、帯の意味が無くなる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// MCP 経由。アプリの外にいるエージェント。
    Ai,
    /// GUI 経由。目の前の人。
    Human,
}

/// `[Human]` は 7 文字で、この 2 つのうち最長。行頭をここで揃える。
const TAG_WIDTH: usize = 7;

impl Actor {
    /// 行頭に付く札。
    pub fn tag(self) -> &'static str {
        match self {
            Actor::Ai => "[AI]",
            Actor::Human => "[Human]",
        }
    }
}

/// 帯の 1 行。**作ったら変えない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandLine {
    seq: u64,
    actor: Actor,
    text: String,
}

impl BandLine {
    pub fn new(seq: u64, actor: Actor, text: impl Into<String>) -> Self {
        Self {
            seq,
            actor,
            text: text.into(),
        }
    }

    /// 帯の中での通し番号。欠番と順序の入れ替わりを検出するために使う。
    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn actor(&self) -> Actor {
        self.actor
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// 人が読む 1 行にする。
    pub fn render(&self) -> String {
        format!(
            "{:<width$} {}",
            self.actor.tag(),
            self.text,
            width = TAG_WIDTH
        )
    }
}
