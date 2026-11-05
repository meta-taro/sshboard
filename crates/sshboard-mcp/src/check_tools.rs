//! 点検表（D72）の MCP 側。**人と同じ点検を、同じ道で回します。**
//!
//! 手順は `run_readonly` と同じく帯に出てから走ります。AI が渡せるのは**点検の id だけ**で、
//! 手順も基準も人が `checks.toml` に書いたものです。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ErrorData};
use serde::Deserialize;
use sshboard_band::Actor;
use tokio_util::sync::CancellationToken;

use crate::server::SshboardMcp;
use crate::ssh_tools::{refuse, until_cancelled};

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CheckId {
    /// `list_checks` が返す点検の id。
    pub check_id: String,
}

fn json(value: &impl serde::Serialize) -> Result<String, ErrorData> {
    serde_json::to_string(value).map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

#[tool_router(router = check_tool_router, vis = "pub")]
impl SshboardMcp {
    /// 人が書いた点検の一覧。**サーバーへは触りません。**
    #[tool(
        description = "List the health checks a human wrote for sshboard. Each check is an ordered \
                       list of steps; each step names a command from list_readonly_commands and the \
                       criterion it must meet (contains / absent text, or a number below / above a \
                       limit). The list is empty until a person writes checks.toml - that is normal. \
                       You cannot add or change checks. Touches no remote server."
    )]
    pub async fn list_checks(&self) -> Result<String, ErrorData> {
        self.show("list_checks").await?;
        let checks = self.engine()?.checks().map_err(refuse)?;
        json(&serde_json::json!({
            "checks": checks.checks(),
            "howToAdd": "A person writes checks in checks.toml next to readonly.toml. Ask them; you cannot add any.",
        }))
    }

    /// 点検を 1 つ回す。**悪しが出ても最後の手順まで回します。**
    #[tool(
        description = "Run one health check by its id from list_checks, on the connection operations \
                       currently go to. Every step runs in order through the same path as \
                       run_readonly, so each one shows on the human's screen. Each step comes back \
                       good, bad (with the reason), or unknown (it could not be measured - not \
                       allowed, not connected, no number in the output); unknown is never counted \
                       as good. The run does not stop at the first bad step. When the same check \
                       ran before on the same connection, each step also carries its previous value."
    )]
    pub async fn run_check(
        &self,
        Parameters(request): Parameters<CheckId>,
        ct: CancellationToken,
    ) -> Result<String, ErrorData> {
        let engine = self.engine()?;
        let report = until_cancelled(&ct, async {
            engine
                .run_check(Actor::Ai, &request.check_id)
                .await
                .map_err(refuse)
        })
        .await?;
        json(&report)
    }
}
