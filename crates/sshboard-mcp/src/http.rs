//! MCP を 127.0.0.1 の Streamable HTTP で出す（decisions D15）。
//!
//! **なぜ stdio ではないか**: stdio の MCP はクライアントがサーバーを起動する形になる。
//! それは GUI とは別のプロセスであり、D8「MCP はアプリ内蔵」と
//! Issue 001「別プロセスを立てない」の両方に反する。
//! アプリ自身が listen すれば、増えるプロセスは 0 本になる。
//!
//! **ここは認証を持っていない。**loopback にしか bind しないが、
//! 同じ端末の別プロセスからは叩ける。Phase 1 で実際の操作を載せる前に
//! トークンを必須にすること。**Phase 0 の `ping` は何も触らないので今は無害。**

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_stream::OutputStream;
use tokio_util::sync::CancellationToken;

use crate::server::SshboardMcp;

/// MCP クライアントが叩くパス。
pub const MCP_PATH: &str = "/mcp";

/// 外部へ出ないよう loopback に固定する。**0.0.0.0 にしないこと。**
const BIND_HOST: [u8; 4] = [127, 0, 0, 1];

/// 立ち上がった MCP の口。
pub struct McpEndpoint {
    addr: SocketAddr,
    cancel: CancellationToken,
}

impl McpEndpoint {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// MCP クライアントへ渡す URL。
    pub fn url(&self) -> String {
        format!("http://{}{MCP_PATH}", self.addr)
    }

    /// 受け付けを止める。
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

/// MCP を立ち上げ、bind が済んでから返る。
///
/// `port` に 0 を渡すと空きポートを OS が選ぶ。
pub async fn serve(
    band: Band,
    stream: Arc<OutputStream>,
    connections_watch: Arc<ConnectionsWatch>,
    port: u16,
    ack_timeout: Duration,
) -> std::io::Result<McpEndpoint> {
    let cancel = CancellationToken::new();

    // SSE ではなく JSON で返す。Phase 0 に server → client の通知は無く、
    // 保持しつづける必要のあるストリームを増やす理由が無い。
    let config = StreamableHttpServerConfig::default()
        .with_json_response(true)
        .with_cancellation_token(cancel.child_token());

    let service: StreamableHttpService<SshboardMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(
                    SshboardMcp::with_ack_timeout(band.clone(), Arc::clone(&stream), ack_timeout)
                        .with_connections_watch(Arc::clone(&connections_watch)),
                )
            },
            Default::default(),
            config,
        );

    // bind まで済ませてから返す。呼び出し側が「まだ開いていない口」を配らないようにする。
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((BIND_HOST, port))).await?;
    let addr = listener.local_addr()?;
    let router = axum::Router::new().nest_service(MCP_PATH, service);

    tokio::spawn({
        let cancel = cancel.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { cancel.cancelled_owned().await })
                .await;
        }
    });

    Ok(McpEndpoint { addr, cancel })
}
