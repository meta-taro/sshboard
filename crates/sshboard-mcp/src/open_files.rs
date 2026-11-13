//! **ファイルのタブを、指定した接続・指定したフォルダで開く**（`show_view files` の続き・2026-11-13）。
//!
//! PdM の依頼（#2035）。人に設定フォルダへファイルを置いてもらう場面で、AI にできたのは
//! タブの切り替えまででした。**どのフォルダかを渡す口が無く**、人が上から手でたどりました。
//!
//! ## 守っていること
//!
//! - **フォルダが在るかを、画面を動かす前に確かめる。**画面だけ動いて空の一覧、を作らない
//! - 開けなかった理由を**別々の答え**にする（`not_found` / `denied` / `file_not_found` /
//!   `busy` / `no_screen`）。人へ頼むことが理由ごとに違うため
//! - **選ぶだけ。**開く・動かす・消すはしない。選ぶ物はフォルダの中の**名前 1 つ**で、パスは受けない
//! - 人が入力中・質問に答えている最中は動かない（判定は画面の側・`screen` と同じ）

use rmcp::ErrorData;
use sshboard_band::Actor;
use sshboard_engine::{EngineError, SshError};
use tokio_util::sync::CancellationToken;

use crate::agent_screen::{ActOutcome, AgentAct};
use crate::server::SshboardMcp;
use crate::ssh_tools::{refuse, until_cancelled, ShowViewRequest};

/// パスと名前の長さの上限。
const MAX_PATH: usize = 4096;
const MAX_NAME: usize = 255;

/// 検査を通った「このフォルダで開いて」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenFiles {
    connection_id: String,
    path: Option<String>,
    select: Option<String>,
}

fn bad(why: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(why.into(), None)
}

fn given(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

impl OpenFiles {
    /// 引数から作る。**`connection_id` / `path` / `select` のどれも無ければ `None`**（今までのタブ切り替え）。
    pub(crate) fn from_request(
        view: &str,
        request: &ShowViewRequest,
    ) -> Result<Option<Self>, ErrorData> {
        let connection_id = given(&request.connection_id);
        let path = given(&request.path);
        let select = given(&request.select);
        if connection_id.is_none() && path.is_none() && select.is_none() {
            return Ok(None);
        }
        if view != "files" {
            return Err(bad(
                "connection_id, path and select are only for the files tab - pass view: files",
            ));
        }
        let Some(connection_id) = connection_id else {
            return Err(bad(
                "connection_id is required with path or select - name the connection",
            ));
        };
        if let Some(path) = &path {
            if path.chars().count() > MAX_PATH {
                return Err(bad(format!(
                    "path is too long (up to {MAX_PATH} characters)"
                )));
            }
        }
        if let Some(name) = &select {
            Self::check_name(name)?;
            if path.is_none() {
                return Err(bad("select needs path - name the folder the file is in"));
            }
        }
        Ok(Some(Self {
            connection_id,
            path,
            select,
        }))
    }

    /// 選ぶ物は**フォルダの中の名前 1 つ**。区切りや `..` で外へ出させない。
    fn check_name(name: &str) -> Result<(), ErrorData> {
        let is_name = !name.contains('/')
            && !name.contains('\\')
            && name != "."
            && name != ".."
            && name.chars().count() <= MAX_NAME;
        if is_name {
            Ok(())
        } else {
            Err(bad(
                "select must be one name inside the folder (no slashes, not . or ..)",
            ))
        }
    }
}

/// 一覧が取れなかった理由を、**AI が人へ何を頼むか決められる答え**にする。
fn why_not_listed(path: &str, error: EngineError) -> ErrorData {
    match error {
        EngineError::Ssh(SshError::NoSuchPath(_)) => bad(format!(
            "not_found: there is no folder {path} on that server - nothing on screen moved"
        )),
        EngineError::Ssh(SshError::PermissionDenied(_)) => bad(format!(
            "denied: this connection may not read {path} - nothing on screen moved. \
             Ask the person; reading it needs a different account or permission."
        )),
        other => refuse(other),
    }
}

impl SshboardMcp {
    /// 確かめてから、画面へ「このフォルダで開いて」と渡す。
    pub(crate) async fn open_files(
        &self,
        open: OpenFiles,
        ct: CancellationToken,
    ) -> Result<String, ErrorData> {
        let engine = self.engine()?;
        let is_open = engine
            .open_connections()
            .await
            .iter()
            .any(|held| held.id == open.connection_id);
        if !is_open {
            return Err(bad(format!(
                "{} is not open - connect to it first",
                open.connection_id
            )));
        }

        if let Some(path) = &open.path {
            let entries = until_cancelled(
                &ct,
                engine.on(&open.connection_id, async {
                    engine
                        .list_dir(Actor::Ai, path)
                        .await
                        .map_err(|error| why_not_listed(path, error))
                }),
            )
            .await?;
            if let Some(name) = &open.select {
                if !entries.iter().any(|entry| &entry.name == name) {
                    return Err(bad(format!(
                        "file_not_found: {path} has nothing named {name} - nothing on screen moved"
                    )));
                }
            }
        }

        let Some(viewer) = self.viewer() else {
            return Err(bad(
                "no_screen: sshboard's window is not open - ask the person to open it",
            ));
        };
        self.show(&format!("show_view files {}", open.connection_id))
            .await?;

        let act = AgentAct::Open {
            connection_id: open.connection_id,
            path: open.path,
            select: open.select,
        };
        match viewer.act(&act).await {
            Ok(ActOutcome::Done) => Ok("done: opened the files tab there".to_string()),
            Ok(ActOutcome::Busy) => Err(bad(
                "busy: the person is typing or answering a question, so the screen did not move. \
                 Try again in a moment.",
            )),
            Ok(other) => Err(ErrorData::internal_error(
                format!("the screen could not open it ({other:?})"),
                None,
            )),
            Err(why) => Err(bad(format!("no_screen: {why}"))),
        }
    }
}
