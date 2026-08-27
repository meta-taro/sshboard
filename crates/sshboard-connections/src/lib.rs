//! 登録された接続の一覧。
//!
//! **秘密をファイルに置きません。**OS ストアの参照名だけを持ちます（D11）。
//! **AI へはホスト名も利用者名も渡しません。**識別子と名前だけです
//! （CLAUDE.md 禁止事項 5）。

mod entry;
mod mark;
mod store;
mod watch;

pub use entry::{ConnectionEntry, ConnectionSummary};
pub use mark::{
    is_connection_color, is_connection_tag, CONNECTION_COLORS, CONNECTION_TAG_MAX_CHARS,
};
pub use store::{default_path, Connections, ConnectionsError, CURRENT_VERSION};
pub use watch::ConnectionsWatch;
