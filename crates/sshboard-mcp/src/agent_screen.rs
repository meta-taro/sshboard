//! **エージェント専用カーソル**（2026-11-12・オーナーの依頼・tadori の仕様が正本）。
//!
//! 人が開いている sshboard の画面の中で、エージェントが「どこを指して何を押したか」を見せる。
//! **人の OS のマウスとキーボードは使いません**（奪わない）。画面の中に専用のカーソルを描くだけです。
//!
//! ## tadori より狭くしてある所（GUI は制限の無い人の口・D22）
//!
//! sshboard の画面には、承認・端末を渡す・ホストを信頼する・パスフレーズ・書き込み先の編集・
//! ダウンロード（D27）が並びます。**押せる物を「人が押せる物すべて」にすると、AI の柵を全部迂回できます。**
//! 仕様の「人の承認・保存の代わりをしない」に従い、ここでは次のように絞ります。
//!
//! - **押す（`click`）は、画面が「エージェントが押してよい」と印を付けた物だけ**（いまはタブ）。
//!   断る物を並べる形にすると、**後から足した釦がすべて押せるようになる**ので、許す物を並べる形にする
//! - **打つ（`type`）は入れない**。打てる欄はどれも人の口（パスフレーズ・接続先・承認の理由）で、
//!   端末の打鍵は錠（D29）を通る `console_type` がある
//! - **人で絞る（`filter`）は無い**（sshboard に人の絞り込みが無いため）
//! - 画面の切り替え（`go`）は、`show_view` と同じ画面の名前だけ。外のページへは行かない
//!
//! 画面に写る接続先（ホスト・利用者名・パス）は**探す対象から外します**（画面側の `agent-cursor.ts`）。
//! 「見つかった／見つからない」の答えだけで、AI が接続先を当てられるからです（CLAUDE.md 禁止事項 4）。
//! 同じ理由で、**見つけた物の文字は AI へ返しません。**

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ErrorData};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::server::SshboardMcp;
use crate::ssh_tools::until_cancelled;

/// 画面に出す文字の上限（吹き出し・探す言葉）。
const MAX_TEXT: usize = 500;
/// 吹き出しを出しておく秒数の範囲。
const MIN_SECONDS: u32 = 1;
const MAX_SECONDS: u32 = 60;

/// 画面へ渡す指示。**検査を通った形だけ**がここに入ります。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum AgentAct {
    /// 指す。押さない。
    Point {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<u32>,
    },
    /// 押す。**画面が印を付けた物だけ。**
    Click {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<u32>,
    },
    /// 画面（タブ）を切り替える。`show_view` と同じ名前。
    Go {
        view: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<u32>,
    },
    /// 一言だけ言う。
    Say {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<u32>,
    },
}

/// 画面の答え。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActOutcome {
    /// 指した・押した・言った。
    Done,
    /// その言葉の物が画面に見えていない（探さない所も含む）。**何も押していない。**
    NotFound,
    /// 見つかったが、エージェントが押してよい物ではない。**何も押していない。**
    NotAllowed,
    /// 人が画面で何かしている最中（問いに答えている・欄に打っている）。**画面を奪わない。**
    Busy,
}

/// `screen` の引数。
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ScreenRequest {
    /// `point`（指す）/ `click`（押す）/ `go`（画面を切り替える）/ `say`（一言）。
    pub action: String,
    /// `point` / `click`：画面に見えている言葉（釦の文字・見出し・タブの名前）。座標では指しません。
    #[serde(default)]
    pub target: Option<String>,
    /// `go`：`connections` / `files` / `console` / `band` / `diag` のどれか。
    #[serde(default)]
    pub view: Option<String>,
    /// 吹き出しの文（500 文字まで）。`say` では必須。
    #[serde(default)]
    pub message: Option<String>,
    /// 吹き出しを出しておく秒数（1〜60）。省けば文の長さで決まります。
    #[serde(default)]
    pub seconds: Option<u32>,
}

fn text(value: &Option<String>, key: &str, required: bool) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return if required {
            Err(format!("{key} is required"))
        } else {
            Ok(None)
        };
    };
    if value.trim().is_empty() {
        return Err(format!("{key} must be non-empty text"));
    }
    if value.chars().count() > MAX_TEXT {
        return Err(format!("{key} is too long (up to {MAX_TEXT} characters)"));
    }
    Ok(Some(value.clone()))
}

fn seconds(value: Option<u32>) -> Result<Option<u32>, String> {
    match value {
        Some(s) if !(MIN_SECONDS..=MAX_SECONDS).contains(&s) => Err(format!(
            "seconds must be from {MIN_SECONDS} to {MAX_SECONDS}"
        )),
        other => Ok(other),
    }
}

/// 受けた引数を、画面へ流してよい指示にする。**おかしければ理由つきで断る。**
pub fn parse_act(request: &ScreenRequest) -> Result<AgentAct, String> {
    let message = text(&request.message, "message", request.action == "say")?;
    let seconds = seconds(request.seconds)?;
    match request.action.as_str() {
        "point" | "click" => {
            let target = text(&request.target, "target", true)?.unwrap_or_default();
            Ok(if request.action == "point" {
                AgentAct::Point {
                    target,
                    message,
                    seconds,
                }
            } else {
                AgentAct::Click {
                    target,
                    message,
                    seconds,
                }
            })
        }
        "go" => {
            let view = text(&request.view, "view", true)?
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            if !crate::view::VIEWS.contains(&view.as_str()) {
                return Err(format!(
                    "there is no view called {view}; use one of {}",
                    crate::view::VIEWS.join(" / ")
                ));
            }
            Ok(AgentAct::Go {
                view,
                message,
                seconds,
            })
        }
        "say" => Ok(AgentAct::Say {
            message: message.unwrap_or_default(),
            seconds,
        }),
        "type" => Err(
            "sshboard's agent cursor does not type. Every field on this screen \
                       belongs to the person (passphrases, connection details, approval reasons); \
                       to type into a terminal, use console_type."
                .to_string(),
        ),
        other => Err(format!(
            "unknown action: {other} (available: point / click / go / say)"
        )),
    }
}

/// 画面の答えを、AI へ返す文にする。**見つけた物の文字は返しません**（接続先を当てさせない）。
fn reply(act: &AgentAct, outcome: ActOutcome) -> Result<String, ErrorData> {
    match outcome {
        ActOutcome::Done => Ok(match act {
            AgentAct::Point { .. } => "pointed at it on the person's screen".to_string(),
            AgentAct::Click { .. } => "pressed it on the person's screen".to_string(),
            AgentAct::Go { view, .. } => format!("asked the person's screen to show {view}"),
            AgentAct::Say { .. } => "said it on the person's screen".to_string(),
        }),
        ActOutcome::NotFound => Err(ErrorData::invalid_params(
            "not found on the screen - nothing was pressed. Name something the person can see \
             (a button, heading or tab). Connection details on screen are never matched.",
            None,
        )),
        ActOutcome::Busy => Err(ErrorData::invalid_params(
            "the person is busy on screen (answering a question or typing) - nothing was \
             changed. Their screen is theirs; try again in a little while.",
            None,
        )),
        ActOutcome::NotAllowed => Err(ErrorData::invalid_params(
            "that is for the person to press, not the agent - nothing was pressed. The agent \
             may only press what the screen marks for agents (the view tabs). Point at it and \
             ask the person instead.",
            None,
        )),
    }
}

#[tool_router(router = screen_tool_router, vis = "pub")]
impl SshboardMcp {
    /// エージェント専用カーソル。**人のマウスとキーボードは使いません。**
    #[tool(
        description = "Show the person what you are doing, inside sshboard's own window, with an \
                       agent-only cursor. Their mouse and keyboard are never used. point moves the \
                       cursor to something they can see (name it by its visible text - a button, \
                       heading or tab; never by coordinates) and can show a message bubble. click \
                       presses it, but only things the screen marks for agents (the view tabs); \
                       approvals, passphrases and connection settings are the person's to press. \
                       go switches the view (connections / files / console / band / diag). say \
                       shows a message. There is no type. If sshboard's window is not open, this \
                       says so."
    )]
    pub async fn screen(
        &self,
        Parameters(request): Parameters<ScreenRequest>,
        ct: CancellationToken,
    ) -> Result<String, ErrorData> {
        let act = parse_act(&request).map_err(|why| ErrorData::invalid_params(why, None))?;

        let Some(viewer) = self.viewer() else {
            return Err(ErrorData::internal_error(
                "sshboard's window is not open - ask the person to open it first".to_string(),
                None,
            ));
        };

        // **帯に出します**（他の道具と同じ）。言葉は載せません —— 載せるのは何をしたかだけ。
        self.show(&format!("screen {}", request.action)).await?;

        let outcome = until_cancelled(&ct, async {
            viewer
                .act(&act)
                .await
                .map_err(|why| ErrorData::internal_error(why, None))
        })
        .await?;
        reply(&act, outcome)
    }
}
