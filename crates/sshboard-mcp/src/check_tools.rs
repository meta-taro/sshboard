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
    /// どの接続で動かすか。`list_connections` / `session_status` の識別子。
    ///
    /// **必ず名指しします。**画面で人が押しているタブには頼りません
    /// （押されたタブへ黙って届いた・2026-11-11）。
    pub connection_id: String,
    /// `list_checks` が返す点検の id。
    pub check_id: String,
}

/// 案の手順 1 つ。**`list_readonly_commands` の id を指すだけ。**
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ProposedStep {
    /// `list_readonly_commands` の id。
    pub readonly: String,
    /// 人に見せる名前。
    pub label: Option<String>,
    /// 出力にこの字が出ていれば良し。
    pub contains: Option<String>,
    /// 出力にこの字が出ていなければ良し。
    pub absent: Option<String>,
    /// 出力の最初の数が、これ未満なら良し。
    pub below: Option<f64>,
    /// 出力の最初の数が、これより大きければ良し。
    pub above: Option<f64>,
}

/// 点検の案。
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ProposedCheck {
    /// 点検の id（既に在るものは上書きできません）。
    pub check_id: String,
    /// 何を見る点検か。人が読んで保存するかを決めます。
    pub description: Option<String>,
    /// 書いた順に回す手順。
    pub steps: Vec<ProposedStep>,
}

impl From<ProposedCheck> for sshboard_readonly::Check {
    fn from(proposed: ProposedCheck) -> Self {
        Self {
            id: proposed.check_id,
            description: proposed.description,
            steps: proposed
                .steps
                .into_iter()
                .map(|step| sshboard_readonly::Step {
                    readonly: step.readonly,
                    label: step.label,
                    expect: sshboard_readonly::Expect {
                        contains: step.contains,
                        absent: step.absent,
                        below: step.below,
                        above: step.above,
                    },
                })
                .collect(),
        }
    }
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

    /// **点検の案を出す。**書きません —— 保存するのは画面の人です（D72）。
    #[tool(
        description = "Propose a new health check to the person. Nothing is written: the proposal \
                       appears on their screen with every step, and only they can save it to \
                       checks.toml. Each step must name an id from list_readonly_commands; give it \
                       a criterion if useful (contains / absent text, or below / above a number - \
                       the first number in the output). Refused before the person sees it if a \
                       step is not on the allowlist, the check id already exists, or another \
                       proposal is still waiting for their answer."
    )]
    pub async fn propose_check(
        &self,
        Parameters(request): Parameters<ProposedCheck>,
    ) -> Result<String, ErrorData> {
        let engine = self.engine()?;
        let id = request.check_id.clone();
        engine
            .propose_check(Actor::Ai, request.into())
            .await
            .map_err(refuse)?;
        json(&serde_json::json!({
            "proposed": id,
            "next": "The proposal is on the person's screen. Only the person can save it; once saved, run it with run_check.",
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
        let report = until_cancelled(
            &ct,
            engine.on(&request.connection_id, async {
                engine
                    .run_check(Actor::Ai, &request.check_id)
                    .await
                    .map_err(refuse)
            }),
        )
        .await?;
        json(&report)
    }
}
