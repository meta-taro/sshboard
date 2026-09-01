//! sshboard の MCP サーバー。**アプリに同居する**（decisions D8）。
//!
//! GUI には依存しない。ここが Tauri を知った瞬間、ヘッドレスでテストできなくなる。

mod capture;
mod http;
mod server;
mod ssh_tools;

pub use capture::{WindowCapture, WindowShot};
pub use http::{new_token, serve, McpEndpoint, ServeParts, MCP_PATH};
pub use server::{
    CaptureWindow, MarkConnection, RegisterConnection, SshboardMcp, DEFAULT_ACK_TIMEOUT,
};
pub use ssh_tools::{
    ConnectionId, HowMany, MaybeConnectionId, OpenConsole, ReadLog, ReadonlyCommandId, RemotePath,
    Search, ServiceName, TypeIntoConsole, UploadFile, WriteFile,
};
