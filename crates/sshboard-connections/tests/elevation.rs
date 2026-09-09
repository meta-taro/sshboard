//! **どうやって権限を得るか**（D48 / Issue #19）。**サーバーを一切使いません。**
//!
//! `operations.toml` は「**どのコマンドを走らせてよいか**」を解きますが、
//! 「**どうやって権限を得るか**」は解いていませんでした。
//!
//! > **AI 側から root 領域を読む道が、現時点でゼロです。**
//!
//! 実機（v0.1.11-alpha.12）で確かめられた事実:
//!
//! ```text
//! $ sudo -n true
//! sudo: パスワードが必要です
//! ```
//!
//! **`sudoers` の道が、そのサーバーでは最初から閉じています。**
//! D46 で「推奨」と書いたものが、**推奨できる状態ですらなかった**ということです。
//!
//! 見張るのは 7 つ。
//!
//! 1. **既定は「上げない」**（書くまで何も変わらない）
//! 2. `sudoers` は**必ず `-n`** ＝ 入力待ちで固まらず、その場で落ちる
//! 3. `ask` は**必ず `-S`** ＝ 標準入力から受ける（端末の入力待ちにしない）
//! 4. `ask` は**必ず `-p ''`** ＝ 促し文句を出さない（stderr を汚さない）
//! 5. **秘密がコマンド行に載らない**（載ると `ps` で他人に見える）
//! 6. **`sudo` で始まらないものは、何も変えない・秘密も要らない**
//! 7. **途中の `sudo` は上げない**（`sh -c '… sudo …'` を勝手に書き換えない）

use sshboard_connections::{elevated, Elevation};

#[test]
fn nothing_changes_until_a_person_chooses_a_way() {
    // **既定は「上げない」。**書くまで、いまと 1 文字も変わりません。
    let asis = elevated("sudo systemctl reload httpd", Elevation::None);

    assert_eq!(asis.command, "sudo systemctl reload httpd");
    assert!(!asis.needs_secret, "既定で秘密を求めている");
}

#[test]
fn the_sudoers_way_never_waits_for_input() {
    // **`-n` が要ります。**付けないと、`sudoers` が効いていないサーバーで
    // **入力待ちのまま固まります**（exec には端末がありません）。
    // 実機で `sudo -n true` が「パスワードが必要です」と即座に返ったのは、
    // **`-n` が付いていたから**です。
    let ran = elevated("sudo systemctl reload httpd", Elevation::Sudoers);

    assert_eq!(ran.command, "sudo -n systemctl reload httpd");
    assert!(!ran.needs_secret, "sudoers なのに秘密を求めている");
}

#[test]
fn the_ask_way_reads_the_secret_from_standard_input() {
    // **`-S` で標準入力から受けます。**コマンド行に置きません。
    // **`-p ''` で促し文句を消します** —— 消さないと `[sudo] password for …` が
    // stderr に混ざり、**AI が「エラーが出た」と読み違えます。**
    let ran = elevated("sudo tail -n 200 /var/log/some.log", Elevation::Ask);

    assert_eq!(ran.command, "sudo -S -p '' tail -n 200 /var/log/some.log");
    assert!(ran.needs_secret, "ask なのに秘密を求めていない");
}

#[test]
fn the_secret_never_appears_on_the_command_line() {
    // **これが芯です。**コマンド行に置くと `ps` で他人に見えます。
    // 組み立てる側が秘密を受け取らない形にしてあるので、**置きようがありません。**
    let ran = elevated("sudo systemctl restart httpd", Elevation::Ask);

    assert!(
        !ran.command.contains("--stdin") && !ran.command.contains('\n'),
        "組み立てた文字列が壊れている: {}",
        ran.command
    );
    // 引数は人が書いた分だけ。**秘密を入れる隙間がありません。**
    assert!(ran.command.ends_with("systemctl restart httpd"));
}

#[test]
fn something_that_is_not_sudo_is_left_alone() {
    // **`sudo` で始まらないものは、何も変えません。**
    // `ask` にしただけで、全部の操作がパスワードを求めるようになったら
    // **人は使うのをやめます。**
    for way in [Elevation::None, Elevation::Sudoers, Elevation::Ask] {
        let ran = elevated("systemctl reload httpd", way);

        assert_eq!(
            ran.command, "systemctl reload httpd",
            "{way:?} で書き換わった"
        );
        assert!(!ran.needs_secret, "{way:?} で秘密を求めている");
    }
}

#[test]
fn a_sudo_in_the_middle_is_not_touched() {
    // **途中の `sudo` は上げません。**引用符の中かもしれず、
    // **人が書いた文字列を、製品が勝手に読み替えない**方を採ります。
    let ran = elevated("sh -c 'sudo systemctl reload httpd'", Elevation::Ask);

    assert_eq!(ran.command, "sh -c 'sudo systemctl reload httpd'");
    assert!(!ran.needs_secret, "途中の sudo で秘密を求めている");
}

#[test]
fn a_person_writes_the_way_in_the_connection_file() {
    // **接続ごとに、人が書きます。**サーバーによって使える道が違うからです。
    #[derive(serde::Deserialize)]
    struct Holder {
        #[serde(default)]
        r#become: Elevation,
    }

    let asked: Holder = toml::from_str("become = \"ask\"\n").expect("読めない");
    assert_eq!(asked.r#become, Elevation::Ask);

    let sudoers: Holder = toml::from_str("become = \"sudoers\"\n").expect("読めない");
    assert_eq!(sudoers.r#become, Elevation::Sudoers);

    // **書かなければ「上げない」。**
    let silent: Holder = toml::from_str("").expect("読めない");
    assert_eq!(silent.r#become, Elevation::None);
}
