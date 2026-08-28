//! 開いている 1 本について、**外へ見せてよい形**。

use serde::Serialize;

/// AI が書ける範囲。**人へも AI へも、同じものを見せる。**
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteAccess {
    /// AI が書けるディレクトリ。空 ＝ AI は書けない。
    pub ai_roots: Vec<String>,
    /// 人が書けるか。**常に true**（PRD §3）。
    /// 値としては自明だが、画面に「人は自由・AI は囲い」と出すために持たせる。
    pub human_unrestricted: bool,
}

/// 開いている接続。**ホスト名も利用者名も入れません**（CLAUDE.md 禁止事項 5）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// 繋がった相手のホスト鍵の指紋。**人が確かめるためのもの。**
    pub fingerprint: String,
    pub host_key_algorithm: String,
    pub write: WriteAccess,
}
