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
