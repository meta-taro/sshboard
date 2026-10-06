//! 端末を**番号で**持つ入れ物と、誰が握るかの規則（D29 / D42 / D60）。
//!
//! ## なぜ切り出したか
//!
//! `engine.rs` が 2414 行になり、端末の規則がその中で SSH と混ざっていました
//! （目安 800 行・product-baseline §8）。**規則そのものは SSH と無関係**です ——
//! D29 の「人の解除が常に勝つ」も、D42 の「握る前に人へ頼む」も、
//! 繋がっていなくても成り立つ話です。混ざっていると、
//! **サーバーが無いと規則を確かめられなくなります。**
//!
//! ## なぜ番号で持つか
//!
//! 接続を鍵にすると、**同じサーバに 2 画面**が表せません。実運用はこうでした ——
//!
//! > レッツエンクリプトで、マルチドメインで、３台同時に延長申請とか作業するから、
//! > テラターム最低２画面はいつも開いている。
//! > **同じサーバに２画面入る場合もある。**
//!
//! 番号は**使い回しません。**閉じた端末の番号が再び出ると、
//! それを覚えていた `read_console` が**黙って別の端末を読みます。**
//!
//! ## 本数は方針、構造ではない
//!
//! [`PER_CONNECTION_LIMIT`] は定数 1 つです。**構造へ焼き込むと、
//! 信用の度合いが変わったときに作り直しになります**（PRD §4-0）。
//! D29 の「全体で 1 本」は `Option<Console>` という構造に焼き込まれていました。

use std::sync::Arc;

use sshboard_band::Actor;
use sshboard_ssh::Console;
use sshboard_stream::OutputStream;

/// **1 つの接続で開ける端末の本数**（方針）。
///
/// **いま 1 なのは、画面が 1 つの接続につき 1 枚しか描けないから**です ——
/// **機構が許しても、画面が映せないものを開かせません。**
/// 映せない端末は「AI からは見えて人からは見えない端末」になり、
/// それは「人が常に見ている」（D29）を壊します。
///
/// 画面分割が入ったところで上げます。**上げるのはこの数字だけ**です。
pub const PER_CONNECTION_LIMIT: usize = 1;

// **0 を書いた版を配らない。**0 にすると端末が 1 本も開けず、
// 端末の面は「開く」を押しても何も起きない板になります。
// **走らせる前に止めます**（D33 で既定ポートに入れたのと同じ作法）。
const _: () = {
    assert!(
        PER_CONNECTION_LIMIT >= 1,
        "PER_CONNECTION_LIMIT が 0 です。端末が 1 本も開けなくなります"
    );
};

/// 端末 1 本について、**画面にも MCP にも出せる事実**。
///
/// **打つ口を持ちません。**持たせると、事実を配るだけのつもりが操作になります。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleFacts {
    /// 端末の呼び名。**使い回しません。**
    pub id: u64,
    /// **どの接続の端末か。**識別子だけです ——
    /// ホスト名も利用者名も入りません（CLAUDE.md 禁止事項 4）。
    pub connection: String,
    /// 握っている側。
    pub holder: Actor,
}

/// その接続の端末 1 本と、握っている側。
///
/// **`console` が `None` のことがあります。**人が許したが、まだ PTY を
/// 立てていない段です（D42）—— 許可は「開く前」に出るので、
/// **握りだけが先に在る時間が必ずあります。**
/// ここを分けずに持っていた頃、「人が許したのに AI が開けない」で
/// 行き詰まりました（2026-10-02）。
pub(crate) struct OpenConsole {
    pub(crate) id: u64,
    pub(crate) connection: String,
    pub(crate) holder: Actor,
    pub(crate) console: Option<Console>,
    /// **この端末だけの出力**（Issue #21）。`console_tail` が読むのはここ。
    ///
    /// 接続ごとの出力とは別です。同じ接続の中に、端末の出力と
    /// 用途別ツール（`run_readonly` / `read_log`）の出力が両方流れます。
    pub(crate) stream: Option<Arc<OutputStream>>,
}

impl OpenConsole {
    /// **PTY が立っているか。**握りだけの段では偽です。
    pub(crate) fn is_up(&self) -> bool {
        self.console.is_some()
    }
}

/// AI が「端末を使いたい」と言っている頼み 1 件（D42 / D60）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsoleRequest {
    /// **どの接続について頼んだか。**名前で持ちます ——
    /// 「いまの宛先」で持つと、人が答えるまでに宛先が動いたとき、
    /// **人が許していない接続の端末が AI へ渡ります。**
    pub(crate) on: String,
    /// 誰が頼んだか。
    pub(crate) by: Actor,
}

/// その接続に端末が何本もあって、**どれの話か決められない。**
///
/// **勝手に 1 本選びません。**推測した端末へ打つのが、いちばん危険です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ambiguous(pub(crate) usize);

/// 開いている端末ぜんぶと、握り・頼み（D29）。
///
/// **ロックは `Engine` が 1 か所で持ちます。**画面と MCP が別々に持つと、
/// 必ず食い違います（D25 で実際に食い違って気づきました）。
#[derive(Default)]
pub(crate) struct Consoles {
    items: Vec<OpenConsole>,
    /// 次に渡す番号のもと。**減りません。**
    last_id: u64,
    /// **AI が「使いたい」と言っている**（D42）。人が答えるまで残ります。
    ///
    /// 積み上げません。**何度頼まれても、人に出る問いは 1 つ**です
    /// （催促で人を疲れさせると、いずれ中身を見ずに許すようになります）。
    pub(crate) request: Option<ConsoleRequest>,
}

impl Consoles {
    /// **立っている端末の事実**を、番号の順に。
    ///
    /// **握りだけ在って立っていないものは出しません。**出すと
    /// 「開いている」と読まれて、打とうとして断られます。
    pub(crate) fn facts(&self) -> Vec<ConsoleFacts> {
        let mut all: Vec<ConsoleFacts> = self
            .items
            .iter()
            .filter(|open| open.is_up())
            .map(|open| ConsoleFacts {
                id: open.id,
                connection: open.connection.clone(),
                holder: open.holder,
            })
            .collect();
        all.sort_by_key(|facts| facts.id);
        all
    }

    pub(crate) fn by_id(&self, id: u64) -> Option<&OpenConsole> {
        self.items.iter().find(|open| open.id == id)
    }

    pub(crate) fn by_id_mut(&mut self, id: u64) -> Option<&mut OpenConsole> {
        self.items.iter_mut().find(|open| open.id == id)
    }

    /// その接続のもの全部（**握りだけのものも含みます**）。
    pub(crate) fn on(&self, connection: &str) -> Vec<&OpenConsole> {
        self.items
            .iter()
            .filter(|open| open.connection == connection)
            .collect()
    }

    /// その接続の**ただ 1 本**。
    ///
    /// 2 本以上あるなら [`Ambiguous`] を返します。**勝手に選びません** ——
    /// 接続の名前だけで指す口（画面がこれを使います）は、
    /// **本数が増えた日に「どれの話か」を言えなくなります。**
    pub(crate) fn the_one_on(&self, connection: &str) -> Result<Option<&OpenConsole>, Ambiguous> {
        let here = self.on(connection);
        match here.len() {
            0 => Ok(None),
            1 => Ok(Some(here[0])),
            many => Err(Ambiguous(many)),
        }
    }

    /// **その相手が握っている、立っている端末。**AI は全体で 1 本だけ（D60）。
    ///
    /// **数えるのは立っている端末だけ**です。許可だけ出ている段を数えると、
    /// **人が許した直後に「既に握っています」と断る**ことになります。
    pub(crate) fn up_held_by(&self, actor: Actor) -> Option<&OpenConsole> {
        self.items
            .iter()
            .find(|open| open.holder == actor && open.is_up())
    }

    /// **許可は出ているが、まだ端末を立てていないもの**（Issue #29）。
    pub(crate) fn may_open(&self, actor: Actor) -> Option<&OpenConsole> {
        self.items
            .iter()
            .find(|open| open.holder == actor && !open.is_up())
    }

    /// もう 1 本開けるか。**方針（[`PER_CONNECTION_LIMIT`]）で決まります。**
    pub(crate) fn room_on(&self, connection: &str) -> bool {
        let up = self
            .items
            .iter()
            .filter(|open| open.connection == connection && open.is_up())
            .count();
        room_for_one_more(up, PER_CONNECTION_LIMIT)
    }

    /// **握りを置く。**端末が立っていなくても置けます（D42 の許可の段）。
    ///
    /// その接続に既に 1 本あるならそれの握りを変え、無ければ新しく番号を取ります。
    /// 返るのは、その端末の番号。
    pub(crate) fn hold(&mut self, connection: &str, actor: Actor) -> u64 {
        if let Some(open) = self
            .items
            .iter_mut()
            .find(|open| open.connection == connection)
        {
            open.holder = actor;
            return open.id;
        }
        self.add(connection, actor)
    }

    /// 握りだけの端末を 1 本足して、**新しい番号**を返す。
    pub(crate) fn add(&mut self, connection: &str, holder: Actor) -> u64 {
        self.last_id += 1;
        let id = self.last_id;
        self.items.push(OpenConsole {
            id,
            connection: connection.to_owned(),
            holder,
            console: None,
            stream: None,
        });
        id
    }

    /// **立てた PTY を載せる。**それまで載っていたものを返します（呼んだ側が閉じる）。
    pub(crate) fn attach(
        &mut self,
        id: u64,
        console: Console,
        stream: Arc<OutputStream>,
    ) -> Option<Console> {
        let Some(open) = self.by_id_mut(id) else {
            // **番号が無いなら、載せる先がありません。**
            // 呼んだ側が閉じられるように、そのまま返します。
            return Some(console);
        };
        let previous = open.console.replace(console);
        open.stream = Some(stream);
        previous
    }

    /// 番号で取り除く。
    pub(crate) fn remove(&mut self, id: u64) -> Option<OpenConsole> {
        let at = self.items.iter().position(|open| open.id == id)?;
        Some(self.items.remove(at))
    }

    /// その接続のものを全部取り除く（切断のとき）。
    pub(crate) fn remove_on(&mut self, connection: &str) -> Vec<OpenConsole> {
        let (gone, kept): (Vec<OpenConsole>, Vec<OpenConsole>) = std::mem::take(&mut self.items)
            .into_iter()
            .partition(|open| open.connection == connection);
        self.items = kept;
        gone
    }
}

/// もう 1 本開けるか、だけを判定する。
///
/// **`Consoles` から切り出してあります。**端末の実体（`russh` のチャネル）が
/// 無いと `Consoles` は組めず、**この判定だけを確かめる手段が無くなる**ためです。
fn room_for_one_more(open: usize, limit: usize) -> bool {
    open < limit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_open_always_has_room() {
        assert!(room_for_one_more(0, 1), "1 本も開いていないのに開けない");
    }

    #[test]
    fn the_last_slot_being_taken_means_no_room() {
        assert!(!room_for_one_more(1, 1), "上限 1 で 2 本目が開けてしまう");
    }

    #[test]
    fn raising_the_policy_is_all_it_takes_to_allow_more() {
        // **これが肝です。**構造ではなく数を変えるだけで増えること。
        assert!(room_for_one_more(1, 3), "上限を上げても 2 本目が開けない");
        assert!(room_for_one_more(2, 3), "上限を上げても 3 本目が開けない");
        assert!(!room_for_one_more(3, 3), "上限 3 で 4 本目が開けてしまう");
    }

    #[test]
    fn a_policy_of_zero_closes_the_door_rather_than_panicking() {
        // 0 を書いた日に**落ちるのではなく閉まる**こと。
        assert!(!room_for_one_more(0, 0), "上限 0 で開けてしまう");
    }

    #[test]
    fn the_shipped_policy_lets_at_least_one_console_open() {
        // **1 本も開けない版を配らない。**定数を触った日の保険。
        assert!(
            room_for_one_more(0, PER_CONNECTION_LIMIT),
            "配る方針が「端末を開けない」になっている: {PER_CONNECTION_LIMIT}"
        );
    }

    #[test]
    fn numbers_are_never_handed_out_twice() {
        // **使い回すと、古い番号を覚えていた read_console が
        // 黙って別の端末を読みます。**
        let mut consoles = Consoles::default();
        let first = consoles.add("a", Actor::Human);
        let second = consoles.add("b", Actor::Human);
        assert_ne!(first, second);

        consoles.remove(first);
        let third = consoles.add("c", Actor::Human);
        assert_ne!(third, first, "閉じた番号が再び出た");
        assert_ne!(third, second);
    }

    #[test]
    fn a_hold_on_the_same_connection_does_not_pile_up() {
        // 許可が 2 回出ても、端末が 2 本に増えない。
        let mut consoles = Consoles::default();
        let first = consoles.hold("a", Actor::Ai);
        let again = consoles.hold("a", Actor::Human);

        assert_eq!(first, again, "同じ接続で番号が増えた");
        assert_eq!(consoles.on("a").len(), 1);
    }

    #[test]
    fn asking_for_the_one_console_says_so_when_there_are_several() {
        // **勝手に 1 本選ばない。**推測した端末へ打つのが、いちばん危険。
        let mut consoles = Consoles::default();
        assert!(matches!(consoles.the_one_on("a"), Ok(None)));

        consoles.add("a", Actor::Human);
        assert!(matches!(consoles.the_one_on("a"), Ok(Some(_))));

        consoles.add("a", Actor::Human);
        assert_eq!(consoles.the_one_on("a").err(), Some(Ambiguous(2)));
    }

    #[test]
    fn a_hold_without_a_pty_is_not_counted_as_open() {
        // **許可だけの段を「開いている」と数えると、
        // 人が許した直後に「既に握っています」と断ることになります。**
        let mut consoles = Consoles::default();
        consoles.hold("a", Actor::Ai);

        assert!(consoles.facts().is_empty(), "立っていない端末が一覧に出た");
        assert!(consoles.up_held_by(Actor::Ai).is_none());
        assert!(consoles.may_open(Actor::Ai).is_some());
        assert!(consoles.room_on("a"), "許可だけで席が埋まった");
    }

    #[test]
    fn disconnecting_takes_only_that_connections_consoles() {
        let mut consoles = Consoles::default();
        consoles.add("a", Actor::Human);
        consoles.add("b", Actor::Human);

        let gone = consoles.remove_on("a");
        assert_eq!(gone.len(), 1);
        assert_eq!(consoles.on("a").len(), 0);
        assert_eq!(consoles.on("b").len(), 1, "関係ない接続まで消した");
    }
}
