//! **GUI と MCP が共有する 1 つの実行体**（PRD §4-1）。
//!
//! ファイル面も端末面も MCP も、**ここを通ります**。
//! ここを迂回した経路を作った瞬間、「裏で見えない SSH セッション」が生まれます。
//! それがこの製品で**最大の危険**です。
//!
//! - 接続は**複数持てます。ただし 1 本残らず画面に出ます**（D25）。
//! - どの操作も、まず帯へ出て、画面が受け取ってから走ります（D16）。
//! - AI の書き込みは接続ごとの囲いの中だけ（D22）。人は制限しません（PRD §3）。
//! - **手元へ落とす側**（`download_file`）に囲いはかかりません。守る相手が
//!   サーバーではなく手元なので、**黙って上書きしない**ことで守ります。

mod engine;
mod error;
mod open;

pub use engine::{Engine, OnConflict};
pub use error::EngineError;
pub use open::{Opened, WriteAccess};

pub use sshboard_ssh::DirEntry;
