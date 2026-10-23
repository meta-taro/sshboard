//! **どうやって権限を得るか**（D48 / Issue #19）。
//!
//! `operations.toml`（D45）は「**どのコマンドを走らせてよいか**」を解きました。
//! **「どうやって権限を得るか」は、解いていませんでした。**
//!
//! 実機（v0.1.11-alpha.12）で確かめられたこと:
//!
//! ```text
//! $ sudo -n true
//! sudo: パスワードが必要です
//! ```
//!
//! D46 は「範囲は `sudoers.d` で OS に強制させる」を**推奨**としましたが、
//! そのサーバーでは `sudoers` の道が**最初から閉じています。**
//! **推奨できる状態ですらなかった**ということです。
//!
//! ## 3 つの道
//!
//! | `become` | どうやって | 秘密 |
//! |---|---|---|
//! | 書かない | 上げない | 要らない |
//! | `sudoers` | `sudo -n` | 要らない（`sudoers.d` が持つ） |
//! | `ask` | `sudo -S -p ''` ＋ 標準入力 | **人がその場で入れる** |
//!
//! ## `ask` で守っていること
//!
//! - **コマンド行に載せません。**載せると `ps` で他人に見えます。
//!   この module は**秘密を受け取りません** —— 置きようがない形にしてあります
//! - **`-p ''` で促し文句を消します。**消さないと `[sudo] password for …` が
//!   stderr へ出て、**AI が「エラーが出た」と読み違えます**
//! - **`-n` を付けません**（`ask` では入力を待たせるのが正しい）
//!
//! ## `sudo` で始まるものだけ
//!
//! 途中の `sudo` は上げません（`sh -c '… sudo …'`）。引用符の中かもしれず、
//! **人が書いた文字列を、製品が勝手に読み替えない**方を採ります。

use serde::{Deserialize, Serialize};

/// この接続で、どうやって権限を上げるか。**接続ごとに人が書きます。**
///
/// サーバーによって使える道が違うため、製品側では決められません。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Elevation {
    /// **上げない。**既定。書くまで、いまと 1 文字も変わりません。
    #[default]
    None,
    /// `sudoers.d` が持っている。**パスワードは要りません。**
    ///
    /// **`-n` を付けます。**付けないと、効いていないサーバーで
    /// **入力待ちのまま固まります**（exec には端末がありません）。
    Sudoers,
    /// **人がその場で入れる。**保存しません（D48）。
    Ask,
}

/// 組み立てた結果。**秘密は入っていません。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elevated {
    /// 実際に打つ文字列。**画面にも記録にも、これがそのまま出ます。**
    pub command: String,
    /// 人にその場で入れてもらう必要があるか。
    pub needs_secret: bool,
}

/// `sudo …` を、この接続の道に合わせた形へ直す。
///
/// **秘密を引数に取りません。**取らない形にしてあるので、
/// **戻り値のコマンド行に秘密が混ざりようがありません。**
pub fn elevated(run: &str, how: Elevation) -> Elevated {
    const SUDO: &str = "sudo ";

    // **`sudo` で始まらないものは、何も変えません。**
    // `ask` にしただけで全部がパスワードを求めるようになったら、人は使うのをやめます。
    let Some(rest) = run.strip_prefix(SUDO) else {
        return Elevated {
            command: run.to_owned(),
            needs_secret: false,
        };
    };

    match how {
        Elevation::None => Elevated {
            command: run.to_owned(),
            needs_secret: false,
        },
        Elevation::Sudoers => Elevated {
            command: format!("sudo -n {rest}"),
            needs_secret: false,
        },
        Elevation::Ask => Elevated {
            command: format!("sudo -S -p '' {rest}"),
            needs_secret: true,
        },
    }
}

/// **サーバーが `sudo` を持っていない形跡か**（2026-09-25）。
///
/// 実運用で 3 時間溶けました。**この製品は `sudo` しか組み立てません。**
/// `sudo` を設定せず `su -` で運用しているサーバーでは、
/// `operations.toml` は**原理的に届きません。**
///
/// ところが人にも AI にも、そうは見えません ——
///
/// ```text
/// sudo: パスワードが与えられませんでした
/// sudo: 1 回パスワード試行を間違えました
/// ```
///
/// **「パスワードを間違えた」と読みます。**実際に、報告した席も、
/// それを受けたこちらも、そう読みました。**入れ直しても永久に通りません。**
///
/// **`sudoers` という字を見ます。**英語（`is not in the sudoers file`）でも
/// 日本語（`sudoers ファイルに…`）でも、この 1 語は残ります。
/// 実行ファイルが無い形も拾います。
pub fn sudo_is_unavailable(stderr: &str) -> bool {
    let said = stderr.to_ascii_lowercase();
    // **`sudoers` は訳されません。**どの言語でも残る 1 語。
    said.contains("sudoers")
        // `sudo` そのものが入っていない。訳語は当てにせず、
        // **`sudo` と「無い」を同じ行で見ます。**
        || (said.contains("sudo")
            && (said.contains("command not found")
                || said.contains("not found")
                || said.contains("見つかりません")
                || said.contains("no such file")))
}

/// **繋いだときの上げ方と、いまファイルに書いてある上げ方がずれているか**
/// （実機の報告・2026-10-23）。
///
/// 上げ方は**繋いだ時点の値を抱えます**（`Live::elevation`）。
/// 接続中に `connections.toml` を書き換えても効きません ——
/// 効くと、**人が画面で見ている状態と食い違う**からです。
///
/// **仕様としては意図どおりですが、欠陥でした。**そう言っている所が
/// 1 つも無かったので、人からは「書いたのに効かない」でした。
/// 実機でまさにそれが起きています ——
/// `become = "ask"` を足したのに `-S` が付かずに走り、
/// sudo が「パスワードを読み取るために端末が必要です」と返しました。
///
/// **繋ぎ直すまで効かない、と言うために使います。**
pub fn drifted(live: Elevation, on_file: Elevation) -> bool {
    live != on_file
}

/// 人へ出す 1 行。**何が効いていて、何をすれば効くか**まで書きます。
///
/// 「変わりました」だけでは、次に何をすればいいか分かりません。
pub fn drift_note(live: Elevation, on_file: Elevation) -> String {
    format!(
        "`become` が書き換わっています（ファイルは {}、いま効いているのは {}）。\
         **繋ぎ直すまで効きません。**",
        name(on_file),
        name(live)
    )
}

fn name(how: Elevation) -> &'static str {
    match how {
        Elevation::None => "none",
        Elevation::Sudoers => "sudoers",
        Elevation::Ask => "ask",
    }
}

#[cfg(test)]
mod drift_tests {
    use super::{drift_note, drifted, Elevation};

    #[test]
    fn the_same_setting_is_not_a_drift() {
        assert!(!drifted(Elevation::Ask, Elevation::Ask));
        assert!(!drifted(Elevation::None, Elevation::None));
    }

    #[test]
    fn adding_ask_while_connected_is_a_drift() {
        // **実機で起きた形。**繋いでから `become = "ask"` を足した。
        assert!(drifted(Elevation::None, Elevation::Ask));
    }

    #[test]
    fn taking_it_away_is_a_drift_too() {
        // 外した側も言います。**効いていないと思っているのに効いている**方が危ない。
        assert!(drifted(Elevation::Ask, Elevation::None));
    }

    #[test]
    fn the_note_says_both_sides_and_what_to_do() {
        // 「変わりました」だけでは、次に何をすればいいか分かりません。
        let note = drift_note(Elevation::None, Elevation::Ask);
        assert!(note.contains("ask"), "{note}");
        assert!(note.contains("none"), "{note}");
        assert!(note.contains("繋ぎ直す"), "{note}");
    }

    #[test]
    fn the_note_carries_no_secret_and_no_host() {
        // **帯と診断は、人が issue へ貼る前提の物**です（禁止事項 4 / §14）。
        let note = drift_note(Elevation::Sudoers, Elevation::Ask);
        for never in ["@", "password", "パスワード", "sudo -S"] {
            assert!(!note.contains(never), "{never} が入っている: {note}");
        }
    }
}
