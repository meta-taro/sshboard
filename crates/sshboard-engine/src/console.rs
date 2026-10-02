//! 端末を**何本でも**持てる入れ物（D29 の書き換え）。
//!
//! ## なぜ移したか
//!
//! それまでは `engine.rs` の `Mutex<ConsoleSlot>` が `Option<Console>` を 1 つ持ち、
//! **「1 本だけ」が構造へ焼き込まれていました。**実機で詰まりました（2026-10-02）——
//!
//! > **３台同時に延長申請とか作業するから、テラターム最低２画面はいつも開いている。**
//! > しかし sshboard でやろうとしたら、**これできるのか！？から始まった**
//!
//! 機構を N 本ぶんにして、**本数は [`CONSOLE_LIMIT`] という方針へ出します。**
//! 信用の度合いが変われば定数を上げるだけで済み、構造の作り直しになりません。
//!
//! ## 出力を 1 本にまとめない
//!
//! 端末 1 本ごとに、**その端末だけの出力**を持ちます。Issue #21 ——
//!
//! > `read_stream` で見分けられない
//!
//! まとめた出力しか無いと、端末が 2 本開いた日に
//! **「どちらが喋ったのか言えない」が確定します。**
//! 共有の出力へも同じものを流します（**画面はそちらを見ています**）。

use std::sync::Arc;

use sshboard_band::Actor;
use sshboard_ssh::Console;
use sshboard_stream::OutputStream;

/// **同時に開ける端末の本数。**
///
/// **方針であって、構造ではありません**（それが D29 の失敗でした）。
/// いま 1 なのは、端末の面が 1 本しか描けないからです ——
/// **機構が許しても、画面が映せないものを開かせません。**
/// 画面の分割（作る順番の 3）が入ったところで上げます。
pub const CONSOLE_LIMIT: usize = 1;

// **0 を書いた版を配らない。**0 にすると端末が 1 本も開けず、
// 端末の面は「開く」を押しても何も起きない板になります。
// **走らせる前に止めます**（D33 で既定ポートに入れたのと同じ作法）。
const _: () = {
    assert!(
        CONSOLE_LIMIT >= 1,
        "CONSOLE_LIMIT が 0 です。端末が 1 本も開けなくなります"
    );
};

/// 端末 1 本について、**画面にも MCP にも出せる事実**。
///
/// **打つ口を持ちません。**持たせると、事実を配るだけのつもりが操作になります。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleFacts {
    /// 端末の呼び名。**使い回しません**（[`Consoles::add`] に理由）。
    pub id: u64,
    /// **どの接続の端末か。**識別子だけです ——
    /// ホスト名も利用者名も入りません（CLAUDE.md 禁止事項 4）。
    pub connection: String,
    /// 握っている側。**誰も握っていないこともあります。**
    pub holder: Option<Actor>,
}

/// 開いている端末 1 本。
pub(crate) struct OpenConsole {
    pub(crate) id: u64,
    pub(crate) connection: String,
    pub(crate) console: Console,
    /// **この端末だけの出力**（Issue #21）。`read_console` が読むのはここ。
    pub(crate) stream: Arc<OutputStream>,
}

/// 開いている端末ぜんぶと、握り（D29）。
///
/// **ロックはここ 1 か所だけが持ちます。**画面と MCP が別々に持つと、
/// 必ず食い違います（D25 で実際に食い違って気づきました）。
#[derive(Default)]
pub(crate) struct Consoles {
    items: Vec<OpenConsole>,
    /// 次に渡す番号のもと。**減りません。**
    last_id: u64,
    /// 握っている側。
    ///
    /// **いまは全体で 1 つです。**端末ごとに持たせるのは、画面が複数本を
    /// 描けるようになってから（作る順番の 3）—— **先に分けると、
    /// 「どれが AI の握っている 1 本か」を画面が言えません**（D29 の止める条件 2）。
    pub(crate) holder: Option<Actor>,
    /// **AI が「使いたい」と言っている**（D42）。人が答えるまで残ります。
    ///
    /// 積み上げません。**何度頼まれても、人に出る問いは 1 つ**です
    /// （催促で人を疲れさせると、いずれ中身を見ずに許すようになります）。
    pub(crate) request: Option<Actor>,
}

impl Consoles {
    /// 開いている端末の事実を、**開いた順に**返します。
    pub(crate) fn facts(&self) -> Vec<ConsoleFacts> {
        self.items
            .iter()
            .map(|open| ConsoleFacts {
                id: open.id,
                connection: open.connection.clone(),
                holder: self.holder,
            })
            .collect()
    }

    /// 番号で引く。
    pub(crate) fn get(&self, id: u64) -> Option<&OpenConsole> {
        self.items.iter().find(|open| open.id == id)
    }

    /// その接続で開いている端末。**いまは接続ごとに 1 本までです。**
    pub(crate) fn on(&self, connection: &str) -> Option<&OpenConsole> {
        self.items.iter().find(|open| open.connection == connection)
    }

    /// **いま開いている端末のうち、最初の 1 本。**
    ///
    /// 番号を取らない古い口（`console_type` など）がこれを使います。
    /// [`CONSOLE_LIMIT`] が 1 のあいだ、これは「その 1 本」と同じ意味です。
    pub(crate) fn first(&self) -> Option<&OpenConsole> {
        self.items.first()
    }

    /// もう 1 本開けるか。
    pub(crate) fn room_for_one_more(&self) -> bool {
        room_for_one_more(self.items.len(), CONSOLE_LIMIT)
    }

    /// 足して、**新しい番号**を返す。
    ///
    /// **番号は使い回しません。**閉じた端末の番号が再び出ると、
    /// それを覚えていた `read_console` が**黙って別の端末を読みます。**
    pub(crate) fn add(
        &mut self,
        connection: String,
        console: Console,
        stream: Arc<OutputStream>,
    ) -> u64 {
        self.last_id += 1;
        let id = self.last_id;
        self.items.push(OpenConsole {
            id,
            connection,
            console,
            stream,
        });
        id
    }

    /// 全部取り出して空にする（止めるとき）。**握りも頼みもここでは触りません。**
    pub(crate) fn drain(&mut self) -> Vec<OpenConsole> {
        std::mem::take(&mut self.items)
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
        // **これが今回の肝です。**構造ではなく数を変えるだけで増えること。
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
            room_for_one_more(0, CONSOLE_LIMIT),
            "配る方針が「端末を開けない」になっている: {CONSOLE_LIMIT}"
        );
    }
}
