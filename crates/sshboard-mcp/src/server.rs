//! アプリに同居する MCP サーバー本体（decisions D8）。
//!
//! **ツールは帯へ載せてからでないと応答を返さない。**
//! 返してから画面が追いつく形にすると、AI は人より先に動けてしまう。
//! それは PRD §4-2 の「誰が触ったかが画面に出る」を満たさない。

use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use sshboard_band::{Actor, Band, DeliveryOutcome};
use sshboard_stream::OutputStream;

/// 帯が受け取りを返すまで待つ上限。
/// 画面が固まっていることを、ここで初めて検出する。
pub const DEFAULT_ACK_TIMEOUT: Duration = Duration::from_secs(2);

/// MCP から来た操作を帯へ流してから実行する殻。
#[derive(Clone)]
pub struct SshboardMcp {
    band: Band,
    /// 追尾している出力。**GUI と同じ 1 本**（PRD §4-1）。
    stream: Arc<OutputStream>,
    ack_timeout: Duration,
    tool_router: ToolRouter<Self>,
}

impl SshboardMcp {
    pub fn new(band: Band, stream: Arc<OutputStream>) -> Self {
        Self::with_ack_timeout(band, stream, DEFAULT_ACK_TIMEOUT)
    }

    pub fn with_ack_timeout(band: Band, stream: Arc<OutputStream>, ack_timeout: Duration) -> Self {
        Self {
            band,
            stream,
            ack_timeout,
            tool_router: Self::tool_router(),
        }
    }

    /// 帯へ 1 行載せ、**画面が受け取るまで待つ。**
    ///
    /// 受け取りが返らないときはツールを失敗させる。見えないまま先へ進ませない。
    async fn show(&self, text: &str) -> Result<(), ErrorData> {
        let delivery = self.band.record(Actor::Ai, text);

        match delivery.wait_acked(self.ack_timeout).await {
            DeliveryOutcome::Delivered => Ok(()),
            // 接続先の情報を混ぜないこと（PRD §8）。ここに出せるのは件数だけ。
            DeliveryOutcome::TimedOut { acked, expected } => Err(ErrorData::internal_error(
                format!(
                    "sshboard did not confirm the operation on screen ({acked}/{expected} \
                     views acknowledged). Refusing to run unseen."
                ),
                None,
            )),
        }
    }
}

#[tool_router(router = tool_router)]
impl SshboardMcp {
    /// Phase 0 の疎通確認用。**サーバーへは繋がない。**
    #[tool(description = "Check that sshboard is reachable. Touches no remote server.")]
    pub async fn ping(&self) -> Result<String, ErrorData> {
        self.show("ping").await?;
        Ok("pong".to_string())
    }

    /// 追尾している出力の末尾を、**素のテキストで**返す。
    ///
    /// GUI には同じ出力が ANSI のまま流れている。**同じ 1 本を面ごとに違う形で出す**
    /// （Issue 005）。
    #[tool(
        description = "Read the plain-text tail of the output sshboard is following. Never contains ANSI escapes."
    )]
    pub async fn read_stream(&self) -> Result<String, ErrorData> {
        self.show("read_stream").await?;
        Ok(self.stream.plain_tail())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SshboardMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
