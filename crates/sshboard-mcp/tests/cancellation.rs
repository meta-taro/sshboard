//! **返らない実行を止められること**（Issue #31 の ③）。
//!
//! 報告はこうでした ——
//!
//! > 呼び出し側でツールを止めても回復しない。
//! > 戻す手段は `/mcp` の Reconnect（中継の再起動）だけ
//!
//! `rmcp` は `notifications/cancelled` を受けると `ct`（取り消しの合図）を
//! 倒してくれます。**使うのはこちらの仕事**です。
//!
//! ## ここで見るいちばん大事なこと
//!
//! **リモートへ触る道具が、1 本残らず合図を受け取ること。**
//!
//! 仕組みは 2026-10 に入っていたのに、**32 本中 4 本にしか付いていませんでした。**
//! 報告で名指しされた `runtime_versions` / `service_status` / `list_directory` は
//! **どれも付いていない側**でした。**付け忘れは、画面にも試験にも出ません。**
//! だから数えます。

use std::fs;
use std::path::Path;

/// **サーバーへ触る道具。**ここに在るものは、取り消しを受け取らなければいけません。
///
/// 手元で終わるもの（一覧・状態・診断・画面の操作）は入れません ——
/// **返らない原因になり得ない**ので、引数を増やすだけになります。
const TOUCHES_THE_SERVER: &[&str] = &[
    "connect",
    "list_directory",
    "stat",
    "read_file",
    "make_directory",
    "upload_file",
    "write_file",
    "run_readonly",
    "run_operation",
    "disk_usage",
    "process_list",
    "network_listen",
    "service_status",
    "read_log",
    "search",
    "runtime_versions",
    "console_open",
    "console_type",
];

fn source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ssh_tools.rs");
    fs::read_to_string(&path).expect("ssh_tools.rs を読めません")
}

/// 道具 1 本の本体を切り出す。**関数の終わりで切ります。**
///
/// **次の `pub async fn` まで、では切れません**（2026-10-23 に外しました）。
/// `impl` の最後の道具だと、**そこから先ぜんぶ**を飲み込み、
/// 後ろの道具が持っている `until_cancelled` を自分の物として数えます。
/// **壊しても落ちない見張り**になっていました。
///
/// 道具は 4 字下げの `}` で終わるので、そこで切ります。
fn body_of(text: &str, name: &str) -> String {
    let at = text
        .find(&format!("pub async fn {name}("))
        .unwrap_or_else(|| panic!("{name} が見つかりません"));
    let end = text[at..]
        .find("\n    }\n")
        .map(|found| at + found)
        .unwrap_or(text.len());
    text[at..end].to_owned()
}

#[test]
fn every_tool_that_touches_the_server_takes_the_cancel_signal() {
    // **付け忘れは、画面にも他の試験にも出ません。**ここで数えます。
    let text = source();
    let mut missing = Vec::new();
    for name in TOUCHES_THE_SERVER {
        if !body_of(&text, name).contains("ct: CancellationToken") {
            missing.push(*name);
        }
    }
    assert!(
        missing.is_empty(),
        "**取り消しを受け取らない道具があります**: {missing:?}\n\
         呼んだ側が止めても止まりません（Issue #31 の ③）。\n\
         `ct: CancellationToken` を引数に足し、`until_cancelled` で包んでください。"
    );
}

#[test]
fn taking_the_signal_is_not_enough_it_has_to_be_used() {
    // **引数に置いただけで使っていない**のが、いちばんたちの悪い形です
    // （型は通る・試験も通る・止まらない）。
    let text = source();
    let mut unused = Vec::new();
    for name in TOUCHES_THE_SERVER {
        let body = body_of(&text, name);
        if body.contains("ct: CancellationToken") && !body.contains("until_cancelled(&ct") {
            unused.push(*name);
        }
    }
    assert!(
        unused.is_empty(),
        "**合図を受け取っているのに使っていません**: {unused:?}"
    );
}

#[test]
fn the_waiting_tool_can_be_cancelled_too() {
    // `await_answer` は**待つのが仕事**です。呼んだ側が止めたのに
    // 300 秒抱え込むのは筋が通りません。
    let body = body_of(&source(), "await_answer");
    assert!(body.contains("until_cancelled(&ct"), "{body}");
}

#[test]
fn the_message_says_the_remote_may_still_be_running() {
    // **止めたのは待つのをやめただけ**で、向こうは走っているかもしれません。
    // そう言わないと、AI は同じものをもう一度走らせます。
    let text = source();
    let at = text
        .find("async fn until_cancelled")
        .expect("見つかりません");
    let note = &text[at..at + 700];
    assert!(note.contains("may still be running"), "{note}");
}
