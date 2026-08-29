//! sshboard の MCP サーバー。**アプリに同居する**（decisions D8）。
//!
//! GUI には依存しない。ここが Tauri を知った瞬間、ヘッドレスでテストできなくなる。

mod http;
mod server;
mod ssh_tools;

pub use http::{new_token, serve, McpEndpoint, MCP_PATH};
pub use server::{MarkConnection, RegisterConnection, SshboardMcp, DEFAULT_ACK_TIMEOUT};
pub use ssh_tools::{ConnectionId, HowMany, RemotePath, UploadFile, WriteFile};
