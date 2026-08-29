//! サーバーに触る MCP ツール。**すべて Engine を通ります**（PRD §4-1）。
//!
//! ここに `run_command(cmd)` 相当を **1 つも置きません**（D3）。
//! 引数で任意の文字列をシェルへ渡す口を作った時点で、危険は「使わない約束」でしか
//! 防げなくなります。
//!
//! 書き込みは接続ごとの囲いの中だけです（D22）。囲いは人が画面で設定します。
//! **AI がパスフレーズを受け取る項目はここにありません**（D14）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ErrorData};
use serde::Deserialize;
use sshboard_band::Actor;
use sshboard_engine::EngineError;

use crate::server::SshboardMcp;

/// 一度に読み書きする上限。**丸ごとメモリに載るため。**
/// 実測でここに当たったら、分割送信を入れて上げる（YAGNI）。
const MAX_BYTES: usize = 8 * 1024 * 1024;

/// 記録を何件返すか。**多すぎると AI の文脈を食う。**
const DEFAULT_DIAGNOSTICS: usize = 40;
const MAX_DIAGNOSTICS: usize = 200;

/// 断り方を MCP の形へ。**接続先を混ぜない**（PRD §8）。
fn refuse(error: EngineError) -> ErrorData {
    match error {
        // 設定漏れ・人にしかできないことは、AI が次に何をすべきか分かる形で返す。
        EngineError::NotConnected
        | EngineError::AlreadyConnected { .. }
        | EngineError::UnknownConnection(_)
        | EngineError::PassphraseNeeded { .. }
        // **指紋を確かめて登録できるのは人だけ。**AI へは「人に頼め」と返す。
        | EngineError::UntrustedHost { .. } => {
            ErrorData::invalid_params(error.to_string(), None)
        }
        other => ErrorData::internal_error(other.to_string(), None),
    }
}

#[tool_router(router = ssh_tool_router, vis = "pub")]
impl SshboardMcp {
    /// 登録済みの接続を開く。**画面に出ます。**
    #[tool(
        description = "Open the SSH connection registered under this id. sshboard holds exactly one connection at a time and the human sees it on screen. Takes no password or passphrase - if the key needs one, a human must enter it in sshboard."
    )]
    pub async fn connect(
        &self,
        Parameters(request): Parameters<ConnectionId>,
    ) -> Result<String, ErrorData> {
        // **パスフレーズは常に None。**AI の経路から秘密を渡す余地を作らない（D14）。
        let opened = self
            .engine()?
            .connect(Actor::Ai, &request.connection_id, None)
            .await
            .map_err(refuse)?;
        render(&opened)
    }

    /// 開いている接続を閉じる。
    #[tool(description = "Close the SSH connection sshboard currently holds.")]
    pub async fn disconnect(&self) -> Result<String, ErrorData> {
        match self.engine()?.disconnect(Actor::Ai).await {
            Some(open) => Ok(format!("closed `{}`", open.id)),
            None => Ok("nothing was open".to_string()),
        }
    }

    /// いま何が開いているか。**サーバーへは触りません。**
    #[tool(
        description = "Report which connection sshboard currently holds open, and where the AI is allowed to write on it. Touches no remote server."
    )]
    pub async fn session_status(&self) -> Result<String, ErrorData> {
        self.show("session_status").await?;
        match self.engine()?.current().await {
            Some(open) => render(&open),
            None => Ok("{\"open\":false}".to_string()),
        }
    }

    /// 何が起きたかの記録。**サーバーへは触りません。**
    ///
    /// **詰まったときに、AI が自分で状況を掴むための口です。**
    /// 「繋がりません」だけ返して終わりにすると、AI は次の一手を選べません。
    #[tool(
        description = "Read sshboard's recent diagnostic log: which stage each attempt reached, why it stopped, and what to do next. Call this first when something failed. Contains no hostnames, usernames, or secrets. Touches no remote server."
    )]
    pub async fn diagnostics(
        &self,
        Parameters(request): Parameters<HowMany>,
    ) -> Result<String, ErrorData> {
        self.show("diagnostics").await?;

        let diag = self.engine()?.diagnostics();
        let limit = request
            .limit
            .unwrap_or(DEFAULT_DIAGNOSTICS)
            .min(MAX_DIAGNOSTICS);

        serde_json::to_string(&serde_json::json!({
            "events": diag.recent(limit),
            "kept": diag.len(),
            // **黙って消さない。**「全部見えている」と誤解させない。
            "droppedBecauseOlderThanTheBuffer": diag.dropped(),
        }))
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
    }

    /// ディレクトリの一覧。
    #[tool(description = "List one directory on the connected server.")]
    pub async fn list_directory(
        &self,
        Parameters(request): Parameters<RemotePath>,
    ) -> Result<String, ErrorData> {
        let entries = self
            .engine()?
            .list_dir(Actor::Ai, &request.path)
            .await
            .map_err(refuse)?;

        let listed: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                serde_json::json!({
                    "name": entry.name,
                    "isDir": entry.is_dir,
                    "size": entry.size,
                })
            })
            .collect();
        serde_json::to_string(&listed)
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))
    }

    /// ファイルを読む。
    ///
    /// **UTF-8 でない中身が実在します**（EUC-JP のログ・Issue 002）。
    /// 読めないバイトは U+FFFD へ置き換えたうえで、**置き換えた事実を伝えます。**
    #[tool(
        description = "Read one file from the connected server as text. Bytes that are not valid UTF-8 are replaced and the reply says so."
    )]
    pub async fn read_file(
        &self,
        Parameters(request): Parameters<RemotePath>,
    ) -> Result<String, ErrorData> {
        let bytes = self
            .engine()?
            .read_file(Actor::Ai, &request.path)
            .await
            .map_err(refuse)?;

        if bytes.len() > MAX_BYTES {
            return Err(ErrorData::invalid_params(
                format!(
                    "file is {} bytes, over sshboard's {MAX_BYTES} byte limit",
                    bytes.len()
                ),
                None,
            ));
        }

        let text = String::from_utf8_lossy(&bytes);
        let was_lossy = matches!(text, std::borrow::Cow::Owned(_));
        serde_json::to_string(&serde_json::json!({
            "path": request.path,
            "bytes": bytes.len(),
            "encodingWasLossy": was_lossy,
            "text": text,
        }))
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
    }

    /// ディレクトリを（親ごと）作る。**囲いの中だけ**（D22）。
    #[tool(
        description = "Create a directory (and missing parents) on the connected server. Only inside the write directories a human configured for that connection."
    )]
    pub async fn make_directory(
        &self,
        Parameters(request): Parameters<RemotePath>,
    ) -> Result<String, ErrorData> {
        self.engine()?
            .ensure_dir(Actor::Ai, &request.path)
            .await
            .map_err(refuse)?;
        Ok(format!("ready: {}", request.path))
    }

    /// 手元のファイルを 1 つ上げる。**囲いの中だけ**（D22）。
    #[tool(
        description = "Upload one local file to the connected server. Only inside the write directories a human configured for that connection. Overwrites the destination if it exists."
    )]
    pub async fn upload_file(
        &self,
        Parameters(request): Parameters<UploadFile>,
    ) -> Result<String, ErrorData> {
        let local = std::path::PathBuf::from(&request.local_path);
        let written = self
            .engine()?
            .upload_file(Actor::Ai, &local, &request.remote_path)
            .await
            .map_err(refuse)?;
        Ok(format!(
            "uploaded {written} bytes to {}",
            request.remote_path
        ))
    }

    /// 中身をその場で書いて上げる。**囲いの中だけ**（D22）。
    ///
    /// 設定ファイルのように、手元に実体が無いものを置くため。
    #[tool(
        description = "Write text to a file on the connected server. Only inside the write directories a human configured for that connection. Overwrites the destination if it exists."
    )]
    pub async fn write_file(
        &self,
        Parameters(request): Parameters<WriteFile>,
    ) -> Result<String, ErrorData> {
        let bytes = request.content.as_bytes();
        if bytes.len() > MAX_BYTES {
            return Err(ErrorData::invalid_params(
                format!(
                    "content is {} bytes, over sshboard's {MAX_BYTES} byte limit",
                    bytes.len()
                ),
                None,
            ));
        }
        let written = self
            .engine()?
            .upload_bytes(Actor::Ai, &request.remote_path, bytes)
            .await
            .map_err(refuse)?;
        Ok(format!("wrote {written} bytes to {}", request.remote_path))
    }
}

/// 開いている接続を JSON にする。**ホストも利用者名も入りません。**
fn render(opened: &sshboard_engine::Opened) -> Result<String, ErrorData> {
    serde_json::to_string(&serde_json::json!({
        "open": true,
        "connection": opened,
    }))
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct HowMany {
    /// 何件まで返すか。省略すると 40 件、上限は 200 件。
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ConnectionId {
    /// `list_connections` が返す識別子。
    pub connection_id: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct RemotePath {
    /// サーバー上の絶対パス。
    pub path: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UploadFile {
    /// 手元のファイルのパス。**sshboard が動いている機械の上。**
    pub local_path: String,
    /// 置き先の絶対パス。**書き込み許可ディレクトリの下だけ。**
    pub remote_path: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct WriteFile {
    /// 置き先の絶対パス。**書き込み許可ディレクトリの下だけ。**
    pub remote_path: String,
    /// 書く中身。
    pub content: String,
}
