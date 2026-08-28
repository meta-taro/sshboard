//! **GUI と MCP が共有する 1 つの実行体**（PRD §4-1）。
//!
//! ファイル面も端末面も MCP も、**ここを通ります**。
//! ここを迂回した経路を作った瞬間、「裏で見えない SSH セッション」が生まれます。
//! それがこの製品で**最大の危険**です。
//!
//! - 開いている接続は **1 本だけ**。
//! - どの操作も、まず帯へ出て、画面が受け取ってから走ります（D16）。
//! - AI の書き込みは接続ごとの囲いの中だけ（D22）。人は制限しません（PRD §3）。

mod engine;
mod error;
mod open;

pub use engine::Engine;
pub use error::EngineError;
pub use open::{Opened, WriteAccess};

pub use sshboard_ssh::DirEntry;
