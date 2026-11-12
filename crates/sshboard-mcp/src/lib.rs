//! sshboard の MCP サーバー。**アプリに同居する**（decisions D8）。
//!
//! GUI には依存しない。ここが Tauri を知った瞬間、ヘッドレスでテストできなくなる。

mod about;
mod agent_screen;
mod capture;
mod channel;
mod check_tools;
mod http;
mod server;
mod ssh_tools;
mod view;

pub use about::{changes_since, releases, Release, CHANGELOG};
pub use agent_screen::{parse_act, ActOutcome, AgentAct, ScreenRequest};
pub use capture::{WindowCapture, WindowShot};
pub use channel::{answered, channel_notice, Waiting, CHANNEL_CAPABILITY, CHANNEL_METHOD};
pub use check_tools::{CheckId, ProposedCheck, ProposedStep};
pub use http::{new_token, serve, McpEndpoint, ServeParts, MCP_PATH};
pub use server::{
    CaptureWindow, MarkConnection, RegisterConnection, SshboardMcp, DEFAULT_ACK_TIMEOUT,
};
pub use ssh_tools::{
    ConnectionId, HowMany, MaybeConnectionId, OpenConsole, ReadLog, ReadonlyCommandId, RemotePath,
    Search, ServiceName, TypeIntoConsole, UploadFile, WriteFile,
};
pub use view::{ShowView, VIEWS};
