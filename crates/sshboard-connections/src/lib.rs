//! 登録された接続の一覧。
//!
//! **秘密をファイルに置きません。**OS ストアの参照名だけを持ちます（D11）。
//! **AI へはホスト名も利用者名も渡しません。**識別子と名前だけです
//! （CLAUDE.md 禁止事項 5）。

mod entry;
mod store;

pub use entry::{ConnectionEntry, ConnectionSummary};
pub use store::{default_path, Connections, ConnectionsError, CURRENT_VERSION};
