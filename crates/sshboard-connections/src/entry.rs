//! 登録された接続 1 件。
//!
//! **秘密をここに置きません。**鍵のパスフレーズもパスワードも、
//! 置くのは **OS ストアの参照（名前）だけ**です（D11 / dbboard ADR-0013 と同じ形）。
//!
//! **ホスト名・利用者名は、この型の中にだけあります。**
//! AI へ渡すのは [`ConnectionSummary`] で、そちらには入りません
//! （CLAUDE.md 禁止事項 5・`list_connections` は識別子だけを返す）。

use serde::{Deserialize, Serialize};

/// 既定の SSH ポート。
const DEFAULT_PORT: u16 = 22;

fn default_port() -> u16 {
    DEFAULT_PORT
}

/// 登録された接続 1 件。**人が GUI で登録するもの。**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEntry {
    /// 機械が使う識別子。**AI が見るのはこれ。**
    pub id: String,
    /// 人が読む名前。**AI が見るのはこれ。**
    pub name: String,

    /// 接続先。**AI には渡さない。**
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// 利用者名。**AI には渡さない。**
    pub user: String,

    /// 秘密鍵のパス。**無ければ ssh-agent を使う**（推奨）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,

    /// 鍵のパスフレーズが入っている **OS ストアの参照名**。
    /// **パスフレーズそのものではありません。**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyring_passphrase_ref: Option<String>,

    /// ホスト鍵の指紋。**固定するとホスト鍵のすり替えを検出できる。**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,

    /// `known_hosts` のパス。無ければ既定の場所を使う。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_hosts: Option<String>,
}

/// **AI へ渡す形。**
///
/// **ホスト名・IP・利用者名・鍵のパスを持ちません。**
/// フィールドを足すときは、それが AI に見えてよいかを必ず確かめること
/// （PRD §8 / CLAUDE.md 禁止事項 5）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
}

impl ConnectionEntry {
    /// AI へ渡してよい形にする。
    pub fn summary(&self) -> ConnectionSummary {
        ConnectionSummary {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }

    /// 参照している OS ストアの名前。
    pub fn keyring_refs(&self) -> Vec<&str> {
        self.keyring_passphrase_ref.as_deref().into_iter().collect()
    }
}
