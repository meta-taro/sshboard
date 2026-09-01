//! 用途別ツールが実際に打つコマンド。**サーバーを一切使いません。**
//!
//! ここは「文字列の組み立て」だけを見ます。**組み立てを間違えると、
//! 引数がコマンドになります。**それはサーバーに繋いでから気づくものではありません。
//!
//! 見張るのは 3 つです。
//!
//! 1. **引数が必ず囲われている**（`nginx; rm -rf /` が 2 本目にならない）
//! 2. **返ってこないコマンドを作らない**（`systemctl status` は既定でページャへ流す）
//! 3. **書き込む語を混ぜない**（読み取りのツールなので）

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{probes, Engine, EngineError};
use sshboard_stream::OutputStream;

#[test]
fn disk_usage_asks_df_in_a_shape_a_person_can_read() {
    let command = probes::disk_usage();

    assert!(command.starts_with("df "), "実際: {command}");
    // `-h` は人が読む単位、`-P` は**行が折り返さない**書式。
    // 折り返すと、長いマウント先で列がずれて読めなくなる。
    assert!(command.contains("-h"), "実際: {command}");
    assert!(command.contains("-P"), "実際: {command}");
}

#[test]
fn process_list_asks_for_every_process_not_just_mine() {
    let command = probes::process_list();

    assert!(command.starts_with("ps "), "実際: {command}");
    // 自分の分だけ見えても、障害調査には使えない。
    assert!(
        command.contains('a') && command.contains('x'),
        "実際: {command}"
    );
}

#[test]
fn network_listen_falls_back_when_ss_is_not_there() {
    // `ss` が無い古い機械は実在します。**そこで行き止まりにしない。**
    let command = probes::network_listen();

    assert!(command.contains("ss "), "実際: {command}");
    assert!(command.contains("netstat"), "実際: {command}");
    assert!(command.contains("||"), "落ちる先が無い: {command}");
}

#[test]
fn service_status_quotes_the_name_so_it_cannot_become_a_command() {
    // **これが本番。**囲えていなければ、2 本目のコマンドが走ります。
    let command = probes::service_status("nginx; rm -rf /").expect("名前はある");

    assert!(
        command.contains("'nginx; rm -rf /'"),
        "囲われていない: {command}"
    );
    // 囲いの外に素の `;` が出ていないこと。
    let outside = command.replace("'nginx; rm -rf /'", "");
    assert!(!outside.contains(';'), "囲いの外に区切りがある: {command}");
}

#[test]
fn service_status_never_waits_for_a_pager() {
    // **`systemctl status` は既定でページャへ流します。**
    // 端末が無い `exec` で走らせると、**返ってきません。**
    let command = probes::service_status("nginx").expect("名前はある");

    assert!(command.contains("--no-pager"), "実際: {command}");
}

#[test]
fn service_status_keeps_a_leading_dash_from_looking_like_an_option() {
    // `-h` という名前のサービスは無いにせよ、**オプションとして読まれる形**を残さない。
    let command = probes::service_status("-h").expect("名前はある");

    let before = command.split("'-h'").next().expect("囲いの前");
    assert!(before.contains("--"), "オプションの終わりが無い: {command}");
}

#[test]
fn service_status_refuses_an_empty_name() {
    // 空で投げると `systemctl status` が**全ユニットを吐きます。**
    // 「押したのに違うものが返る」になるので、ここで断る。
    assert!(probes::service_status("").is_err());
    assert!(probes::service_status("   ").is_err());
}

#[test]
fn read_log_quotes_the_path_and_counts_the_lines() {
    let command = probes::read_log("/var/log/messages", 200).expect("パスはある");

    assert!(command.starts_with("tail "), "実際: {command}");
    assert!(command.contains("-n 200"), "実際: {command}");
    assert!(command.contains("'/var/log/messages'"), "実際: {command}");
}

#[test]
fn read_log_quotes_a_path_that_tries_to_be_a_command() {
    let command = probes::read_log("/var/log/x; rm -rf /", 10).expect("パスはある");

    assert!(
        command.contains("'/var/log/x; rm -rf /'"),
        "実際: {command}"
    );
    let outside = command.replace("'/var/log/x; rm -rf /'", "");
    assert!(!outside.contains(';'), "囲いの外に区切りがある: {command}");
}

#[test]
fn read_log_refuses_an_empty_path() {
    assert!(probes::read_log("", 10).is_err());
}

#[test]
fn read_log_holds_the_line_count_inside_something_sane() {
    // 0 行は意味が無く、青天井は**丸ごとメモリに載ります。**
    let none = probes::read_log("/var/log/messages", 0).expect("パスはある");
    let huge = probes::read_log("/var/log/messages", 10_000_000).expect("パスはある");

    assert!(none.contains("-n 1"), "実際: {none}");
    assert!(
        huge.contains(&format!("-n {}", probes::MAX_LOG_LINES)),
        "実際: {huge}"
    );
}

/// 1 件も繋いでいない Engine。
fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn a_missing_service_name_is_refused_before_anything_reaches_a_server() {
    // **順番が肝です。**「繋がっていません」より先に「名前がありません」を返す。
    // 逆だと、繋がった瞬間に全ユニットが返る作りでも、テストが気づけません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let error = engine
        .service_status(Actor::Ai, "")
        .await
        .expect_err("空の名前が通った");

    assert!(
        matches!(error, EngineError::BadArgument(_)),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_probe_with_everything_it_needs_stops_at_the_missing_connection() {
    // 引数が揃っているものは断られない。**繋いでいないから止まる**のであって、
    // 引数の検査で止まっているのではない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let error = engine
        .disk_usage(Actor::Ai)
        .await
        .expect_err("繋いでいないので通らないはず");

    assert!(
        matches!(error, EngineError::NotConnected),
        "実際: {error:?}"
    );
}

#[test]
fn no_probe_carries_a_word_that_changes_the_server() {
    // **読み取りのツールです。**書く語が紛れ込んでいないことを、ここで固定する。
    let all = vec![
        probes::disk_usage(),
        probes::process_list(),
        probes::network_listen(),
        probes::service_status("nginx").expect("名前はある"),
        probes::read_log("/var/log/messages", 10).expect("パスはある"),
    ];

    for command in all {
        for forbidden in [
            "rm ", "mv ", "chmod", "chown", "kill", "restart", "> ", ">>",
        ] {
            assert!(
                !command.contains(forbidden),
                "`{forbidden}` が入っている: {command}"
            );
        }
    }
}
