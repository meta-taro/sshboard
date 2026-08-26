//! 1 本の出力を、GUI へは生のまま・MCP へは素のテキストで流す。
//!
//! **同じ出力を 2 回実行しない**（PRD §4-1）。

mod plain;
mod stream;

pub use plain::PlainFilter;
pub use stream::{OutputStream, StreamStopped, PLAIN_TAIL_LIMIT};

/// 購読が追いつかなかった / 閉じたことを表す。利用側が tokio を直接知らずに済むように出す。
pub use tokio::sync::broadcast::error::RecvError;
