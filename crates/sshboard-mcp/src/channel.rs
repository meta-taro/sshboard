//! **人が答えたことを、AI のセッションへ押し出す**（Issue #29）。
//!
//! Claude Code は、`claude/channel` を名乗った MCP の口からの
//! `notifications/claude/channel` を、**入力待ちのセッションにも届けます**（研究段階の機能）。
//! 割符が 2026-10 から同じ形で使っています。
//!
//! # 起こすのは「解けたとき」だけ
//!
//! 新しく問いが立っただけでは起こしません —— 問うたのは AI 自身で、知っています。
//! 起こすのは、**人が画面で答えて、問いが消えたとき**です。
//!
//! # 害の無い形
//!
//! channels を付けずに立ち上げた Claude Code は、この知らせを黙って捨てます。
//! ほかのクライアントにとっても、知らない通知は読み捨てるものです。
//! **接続先は載せません**（禁止事項 4）—— 何の問いが解けたかだけを言います。

use serde_json::Value;

/// Claude Code が受け取る通知の名前。
pub const CHANNEL_METHOD: &str = "notifications/claude/channel";

/// この口が名乗る機能（`capabilities.experimental` の鍵）。
pub const CHANNEL_CAPABILITY: &str = "claude/channel";

/// いま人に尋ねているもの。**種類ごとに在るか無いかだけ。**
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Waiting {
    pub console: bool,
    pub passphrase: bool,
    pub host_key: bool,
    pub operation: bool,
}

/// 前に在って、いま無いもの ＝ **人が答えたもの。**
#[must_use]
pub fn answered(before: Waiting, after: Waiting) -> Vec<&'static str> {
    [
        ("console", before.console && !after.console),
        ("passphrase", before.passphrase && !after.passphrase),
        ("host_key", before.host_key && !after.host_key),
        ("operation", before.operation && !after.operation),
    ]
    .into_iter()
    .filter_map(|(kind, gone)| gone.then_some(kind))
    .collect()
}

/// 押し出す中身。**解けたものが無ければ `None`**（起こさない）。
#[must_use]
pub fn channel_notice(kinds: &[&str]) -> Option<Value> {
    if kinds.is_empty() {
        return None;
    }
    let list = kinds.join(",");
    Some(serde_json::json!({
        "content": format!(
            "The person answered on sshboard's screen ({list}). \
             Call pending_status to see the outcome, then retry the call that was waiting."
        ),
        "meta": { "answered": list },
    }))
}
