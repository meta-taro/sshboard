//! sshboard の MCP サーバー。**アプリに同居する**（decisions D8）。
//!
//! GUI には依存しない。ここが Tauri を知った瞬間、ヘッドレスでテストできなくなる。

mod http;
mod server;

pub use http::{serve, McpEndpoint, MCP_PATH};
pub use server::{RegisterConnection, SshboardMcp, DEFAULT_ACK_TIMEOUT};
