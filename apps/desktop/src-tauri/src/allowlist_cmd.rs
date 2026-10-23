//! **AI が走らせられることを、人へ見せる**（2026-10-23）。
//!
//! ## なぜ要るか
//!
//! **人だけが見られませんでした。**実行体は `readonly.toml` と
//! `operations.toml` を読めて、AI も `list_readonly_commands` /
//! `list_operations` で読めるのに、**画面へ出す口が 1 つもありませんでした。**
//!
//! 本番（2026-10-23）で詰まったのはここです ——
//! 人は書く場所を探して見つけられず、**操作は sshboard の外で行われました。**
//! 断りに置き場を書くようにしましたが（`bab508c`）、
//! **書いた物が入ったかを確かめる所が無ければ、同じ所で止まります。**
//!
//! ## 走らせません
//!
//! **読むだけの口です。**ここから走らせる道は作りません ——
//! 走らせるのは `run_operation`（AI が id を渡し、人が画面で許す）です。
//!
//! ## 画面にしかありません
//!
//! MCP には出しません。AI は既に `list_operations` で読めます。
//! **同じ物を 2 つの口から出すと、どちらを直したか分からなくなります。**

use std::sync::Arc;

use sshboard_engine::Engine;
use tauri::State;

/// 許可された 1 本。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedLine {
    /// AI が渡せる唯一の値。
    pub id: String,
    /// 実際に走る文字列。**人が書いたものが、そのまま走ります。**
    ///
    /// **サーバー側の道とドメインが入ります**（`/var/www/html`・`-d example.com`）——
    /// 画面では伏せられる印を付けます（禁止事項 4）。
    pub run: String,
    /// 何をするものか。読むだけの側では書かない人もいます。
    pub why: Option<String>,
    /// 1 時間あたりの上限。**読むだけの側にはありません**（`None`）。
    pub max_per_hour: Option<u32>,
}

/// 一覧 1 つ分。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedList {
    /// 置き場。**本番で見つけられなかったのがここです。**
    pub path: String,
    pub lines: Vec<AllowedLine>,
    /// 読めなかった理由。
    ///
    /// **`None` と「0 本」は別です。**書き間違いを 0 本と出すと、
    /// 人は「書いたのに断られる」所で止まり、**原因が書き間違いだと気づけません。**
    pub trouble: Option<String>,
}

/// 両方まとめて。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistOverview {
    /// `readonly.toml`。
    pub reading: AllowedList,
    /// `operations.toml`。**状態を変える側。**
    pub changing: AllowedList,
}

/// 両方の一覧を画面へ返す。**走らせません。**
#[tauri::command]
pub fn allowlist_overview(engine: State<'_, Arc<Engine>>) -> AllowlistOverview {
    // **読めなくても返します。**`Result` で包むと、片方が壊れているだけで
    // **もう片方も見えなくなります** —— 人はそこで直す手がかりを失います。
    let reading = match engine.readonly_commands() {
        Ok(found) => AllowedList {
            path: engine.readonly_path().display().to_string(),
            lines: found
                .into_iter()
                .map(|one| AllowedLine {
                    id: one.id,
                    run: one.run,
                    why: one.description,
                    max_per_hour: None,
                })
                .collect(),
            trouble: None,
        },
        Err(error) => AllowedList {
            path: engine.readonly_path().display().to_string(),
            lines: Vec::new(),
            trouble: Some(error.to_string()),
        },
    };

    let changing = match engine.operations() {
        Ok(found) => AllowedList {
            path: engine.operations_path().display().to_string(),
            lines: found
                .all()
                .iter()
                .map(|one| AllowedLine {
                    id: one.id.clone(),
                    run: one.run.clone(),
                    why: Some(one.description.clone()),
                    max_per_hour: Some(one.max_per_hour),
                })
                .collect(),
            trouble: None,
        },
        Err(error) => AllowedList {
            path: engine.operations_path().display().to_string(),
            lines: Vec::new(),
            trouble: Some(error.to_string()),
        },
    };

    AllowlistOverview { reading, changing }
}
