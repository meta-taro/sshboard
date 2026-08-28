//! MCP を 127.0.0.1 の Streamable HTTP で出す（decisions D15）。
//!
//! **なぜ stdio ではないか**: stdio の MCP はクライアントがサーバーを起動する形になる。
//! それは GUI とは別のプロセスであり、D8「MCP はアプリ内蔵」と
//! Issue 001「別プロセスを立てない」の両方に反する。
//! アプリ自身が listen すれば、増えるプロセスは 0 本になる。
//!
//! **トークンを必須にしている**（D23）。loopback にしか bind しないが、
//! 同じ端末の別プロセスからは叩ける。読むだけの頃はそれでも無害だったが、
//! **いまは書き込みが載っている。**起動ごとに変わる合言葉を知らないと 1 本も通らない。

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Request;
use axum::http::{header::AUTHORIZATION, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;
use tokio_util::sync::CancellationToken;

use crate::server::SshboardMcp;

/// MCP クライアントが叩くパス。
pub const MCP_PATH: &str = "/mcp";

/// 外部へ出ないよう loopback に固定する。**0.0.0.0 にしないこと。**
const BIND_HOST: [u8; 4] = [127, 0, 0, 1];

/// 合言葉の長さ（バイト）。**16 進で 64 文字になる。**
const TOKEN_BYTES: usize = 32;

/// 立ち上がった MCP の口。
pub struct McpEndpoint {
    addr: SocketAddr,
    token: String,
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

    /// この起動で有効な合言葉。**`Authorization: Bearer <token>` で送る。**
    ///
    /// **アプリを閉じると無効になります。**保存する値ではありません。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 受け付けを止める。
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

/// MCP を立ち上げ、bind が済んでから返る。
///
/// `port` に 0 を渡すと空きポートを OS が選ぶ。
/// `engine` が `None` なら、サーバーへ触るツールが正直に断るだけで、
/// 帯・出力・接続一覧のツールは動く。
pub async fn serve(
    band: Band,
    stream: Arc<OutputStream>,
    connections_watch: Arc<ConnectionsWatch>,
    engine: Option<Arc<Engine>>,
    port: u16,
    ack_timeout: Duration,
) -> std::io::Result<McpEndpoint> {
    let cancel = CancellationToken::new();
    let token = new_token();

    // SSE ではなく JSON で返す。server → client の通知はまだ無く、
    // 保持しつづける必要のあるストリームを増やす理由が無い。
    let config = StreamableHttpServerConfig::default()
        .with_json_response(true)
        .with_cancellation_token(cancel.child_token());

    let service: StreamableHttpService<SshboardMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                let mut server =
                    SshboardMcp::with_ack_timeout(band.clone(), Arc::clone(&stream), ack_timeout)
                        .with_connections_watch(Arc::clone(&connections_watch));
                if let Some(engine) = engine.clone() {
                    server = server.with_engine(engine);
                }
                Ok(server)
            },
            Default::default(),
            config,
        );

    // bind まで済ませてから返す。呼び出し側が「まだ開いていない口」を配らないようにする。
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((BIND_HOST, port))).await?;
    let addr = listener.local_addr()?;
    let router = axum::Router::new()
        .nest_service(MCP_PATH, service)
        .layer(middleware::from_fn({
            let expected = Arc::new(token.clone());
            move |request, next| require_token(Arc::clone(&expected), request, next)
        }));

    tokio::spawn({
        let cancel = cancel.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { cancel.cancelled_owned().await })
                .await;
        }
    });

    Ok(McpEndpoint {
        addr,
        token,
        cancel,
    })
}

/// 合言葉が合っていなければ、**何もせず 401**（D23）。
async fn require_token(expected: Arc<String>, request: Request, next: Next) -> Response {
    let presented = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("");

    if !same_secret(presented, &expected) {
        // **何が違うのかを返さない。**総当たりの手掛かりを渡さない。
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(axum::body::Body::from(
                "sshboard MCP requires the token shown in the app \
                 (Authorization: Bearer <token>).",
            ))
            .expect("固定の応答を組み立てられない");
    }
    next.run(request).await
}

/// 長さと中身を、**途中で打ち切らずに**比べる。
fn same_secret(presented: &str, expected: &str) -> bool {
    let (a, b) = (presented.as_bytes(), expected.as_bytes());
    // 長さが違えば不一致だが、比較そのものは同じ回数だけ回す。
    let mut difference = (a.len() ^ b.len()) as u8;
    for index in 0..a.len().max(b.len()) {
        let left = a.get(index).copied().unwrap_or(0);
        let right = b.get(index).copied().unwrap_or(0);
        difference |= left ^ right;
    }
    difference == 0
}

/// 起動ごとの合言葉。**OS の乱数から作る。**
fn new_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    // 取れなければ立ち上がらない方がよい。**弱い合言葉で開けたことにしない。**
    getrandom::fill(&mut bytes).expect("OS の乱数を読めません");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{new_token, same_secret, TOKEN_BYTES};

    #[test]
    fn two_tokens_from_the_same_process_are_not_the_same() {
        // 同じ値が出るなら、合言葉として意味が無い。
        assert_ne!(new_token(), new_token());
        assert_eq!(new_token().len(), TOKEN_BYTES * 2);
    }

    #[test]
    fn a_prefix_of_the_token_is_not_accepted() {
        // 前方一致で通ると、1 文字ずつ当てられる。
        let token = new_token();
        assert!(same_secret(&token, &token));
        assert!(!same_secret(&token[..10], &token));
        assert!(!same_secret("", &token));
        assert!(!same_secret(&format!("{token}x"), &token));
    }
}
