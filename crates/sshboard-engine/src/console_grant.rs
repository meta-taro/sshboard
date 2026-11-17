//! **期限付きの端末の許可**（2026-11-16）。
//!
//! ## なぜ要るか
//!
//! 時間で課金される測定用の機械（3 時間で自分から消える）へ AI が繋ぐたび、
//! 人が画面で［許可］を押すまで、**課金中の機械が止まったまま待ちました**
//! （起動から測定まで 6 分・別の日は 30 分）。人が画面を見ていない時間は、
//! そのまま払うだけの時間になります。
//!
//! そこで、**人が先に**「いまから N 時間は聞かずに通す」を出しておけるようにします。
//!
//! ## 緩めないもの
//!
//! - **出すのも外すのも人だけ。**AI が自分に出せたら、許可の意味がありません（D42）
//! - **期限を過ぎたら、元どおり尋ねる。**上限は [`MAX_CONSOLE_GRANT`]。
//!   超える期限は**縮めずに断ります** —— 黙って縮めると、人は自分の出した期限を誤って覚えます
//! - **「新しい接続」の許可は、出した時点で在った接続には効かない。**
//!   人がふだん使っている接続（本番を含む）は、許可を出す前から一覧に在ります。
//!   測定のたびに AI が登録する機械は、許可を出した**あと**に増えます
//! - **人が握っている端末は奪わない。**許可は「尋ねない」だけで、「割り込む」ではありません
//! - **保存しません。**アプリを終えれば消えます。［AI を止める］でも消えます
//! - **AI が握れるのは全体で 1 本**（D60）と、人の［取り返す］（D29）はそのまま効きます

use std::collections::BTreeSet;

use tokio::time::{Duration, Instant};

/// **許可の期限の上限。**丸 1 日。
///
/// 測定の機械は 3 時間で消えますが、夜をまたいで何台も立てることがあります。
/// **それより長い許可は、出したことを人が忘れます。**
pub const MAX_CONSOLE_GRANT: Duration = Duration::from_secs(24 * 60 * 60);

/// **どの接続に効くか。**人が選びます。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantScope {
    /// この接続だけ。
    Connection(String),
    /// **許可を出したあとに一覧へ増えた接続だけ。**
    ///
    /// 出した時点で在った接続には効きません。
    NewConnections,
}

impl GrantScope {
    /// 記録に残す言い方。**接続の識別子だけ**を出します（禁止事項 4）。
    pub fn describe(&self) -> String {
        match self {
            GrantScope::Connection(id) => format!("接続 {id} だけ"),
            GrantScope::NewConnections => "これから増える接続だけ".to_owned(),
        }
    }
}

/// 画面と MCP に出せる、いまの許可の事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFacts {
    pub scope: GrantScope,
    /// **残りの時間。**時刻ではなく長さで返します（手元の時計と食い違わないように）。
    pub remaining: Duration,
}

/// 人が出した許可 1 件。**同時に持つのは 1 件だけ**（出し直すと置き換わります）。
#[derive(Debug, Clone)]
pub(crate) struct ConsoleGrant {
    scope: GrantScope,
    /// `NewConnections` のとき、**許可を出した時点で在った接続**。
    existing: BTreeSet<String>,
    until: Instant,
}

impl ConsoleGrant {
    /// 許可を作る。`existing` は**いま一覧に在る接続の識別子**。
    pub(crate) fn new(
        scope: GrantScope,
        existing: BTreeSet<String>,
        for_how_long: Duration,
        now: Instant,
    ) -> Self {
        Self {
            scope,
            existing,
            until: now + for_how_long,
        }
    }

    /// まだ期限の内か。
    pub(crate) fn is_live(&self, now: Instant) -> bool {
        now < self.until
    }

    /// その接続に効くか。**期限を過ぎていれば、どこにも効きません。**
    pub(crate) fn covers(&self, on: &str, now: Instant) -> bool {
        if !self.is_live(now) {
            return false;
        }
        match &self.scope {
            GrantScope::Connection(id) => id == on,
            GrantScope::NewConnections => !self.existing.contains(on),
        }
    }

    /// **「これから増える接続」の許可**が、その接続に効くか（2026-11-16）。
    ///
    /// 使い捨ての機械の**初めてのホスト鍵**を、人に聞かずに受けるかの判断に使います。
    /// 1 台ずつの許可（`Connection`）では受けません —— 人が知っている接続の鍵が
    /// 変わったなら、それは確かめるべきことだからです。
    pub(crate) fn covers_new(&self, on: &str, now: Instant) -> bool {
        matches!(self.scope, GrantScope::NewConnections) && self.covers(on, now)
    }

    pub(crate) fn facts(&self, now: Instant) -> GrantFacts {
        GrantFacts {
            scope: self.scope.clone(),
            remaining: self.until.saturating_duration_since(now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: Duration = Duration::from_secs(3600);

    fn existing(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn only_a_new_connections_grant_vouches_for_a_first_host_key() {
        let now = Instant::now();
        let fresh = ConsoleGrant::new(GrantScope::NewConnections, existing(&["prod"]), HOUR, now);
        assert!(fresh.covers_new("vm-1", now));
        assert!(
            !fresh.covers_new("prod", now),
            "前から在る接続の鍵まで受けた"
        );
        assert!(!fresh.covers_new("vm-1", now + HOUR), "期限後も受けた");
        let one = ConsoleGrant::new(
            GrantScope::Connection("vm-1".into()),
            existing(&[]),
            HOUR,
            now,
        );
        assert!(!one.covers_new("vm-1", now), "1 台の許可で鍵を受けた");
    }

    #[test]
    fn a_connection_grant_covers_only_that_connection() {
        let now = Instant::now();
        let grant = ConsoleGrant::new(
            GrantScope::Connection("vm".into()),
            existing(&["vm", "prod"]),
            HOUR,
            now,
        );
        assert!(grant.covers("vm", now));
        assert!(!grant.covers("prod", now));
    }

    #[test]
    fn a_new_connections_grant_skips_what_was_there_when_it_was_given() {
        let now = Instant::now();
        let grant = ConsoleGrant::new(GrantScope::NewConnections, existing(&["prod"]), HOUR, now);
        assert!(grant.covers("vm", now));
        assert!(!grant.covers("prod", now));
    }

    #[test]
    fn nothing_is_covered_at_or_after_the_deadline() {
        let now = Instant::now();
        let grant = ConsoleGrant::new(GrantScope::NewConnections, existing(&[]), HOUR, now);
        assert!(!grant.covers("vm", now + HOUR));
        assert_eq!(grant.facts(now + HOUR * 2).remaining, Duration::ZERO);
    }
}
