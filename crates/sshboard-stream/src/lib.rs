//! 1 本の出力を、GUI へは生のまま・MCP へは素のテキストで流す。
//!
//! **同じ出力を 2 回実行しない**（PRD §4-1）。

mod plain;
mod stream;

pub use plain::PlainFilter;
pub use stream::{OutputStream, StreamStopped};
