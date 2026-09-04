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
        // **許可リストに足せるのは人だけ**（D3）。AI へは「人に頼め」と返す。
        | EngineError::NotAllowed { .. }
        | EngineError::Allowlist(_)
        // 値が足りないだけ。**AI が直せる。**
        | EngineError::BadArgument(_)
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
        description = "Open the SSH connection registered under this id. sshboard can hold several connections open at once - opening one does not close the others, and every one of them appears on screen for the human. The connection just opened becomes the target of subsequent file and command operations; use focus_connection to point them at a different open one, and session_status to see every connection currently held open. Takes no password or passphrase - if the key needs one, a human must enter it in sshboard."
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

    /// 開いている接続を閉じる。**省略するといまの宛先。**
    #[tool(
        description = "Close one open SSH connection. Omit connectionId to close the one operations currently go to."
    )]
    pub async fn disconnect(
        &self,
        Parameters(request): Parameters<MaybeConnectionId>,
    ) -> Result<String, ErrorData> {
        match self
            .engine()?
            .disconnect(Actor::Ai, request.connection_id.as_deref())
            .await
        {
            Some(open) => Ok(format!("closed `{}`", open.id)),
            None => Ok("nothing was open".to_string()),
        }
    }

    /// 操作の宛先を変える。**開いているものの中からしか選べません。**
    #[tool(
        description = "Point subsequent file and command operations at one of the already-open connections. sshboard can hold several at once; this chooses which one they go to."
    )]
    pub async fn focus_connection(
        &self,
        Parameters(request): Parameters<ConnectionId>,
    ) -> Result<String, ErrorData> {
        let opened = self
            .engine()?
            .focus(&request.connection_id)
            .await
            .map_err(refuse)?;
        render(&opened)
    }

    /// いま何が開いているか。**サーバーへは触りません。**
    #[tool(
        description = "List every connection sshboard currently holds open, where the AI may write on each, and which one file and command operations currently go to. Touches no remote server."
    )]
    pub async fn session_status(&self) -> Result<String, ErrorData> {
        self.show("session_status").await?;

        let engine = self.engine()?;
        let open = engine.open_connections().await;
        let active = engine.active().await;

        serde_json::to_string(&serde_json::json!({
            // **開いているものは 1 本残らずここに出ます**（D25）。
            "open": open,
            // 操作がどれへ行くか。`focus_connection` で変えられる。
            "operationsGoTo": active.map(|open| open.id),
        }))
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
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

    /// 1 件の属性。
    #[tool(
        description = "Look at one file or directory on the connected server: size, permissions \
                       (octal), last modified time and owner ids. Use this when a read failed - \
                       the permissions usually say why. Owner ids are numbers; sshboard does not \
                       resolve them to names."
    )]
    pub async fn stat(
        &self,
        Parameters(request): Parameters<RemotePath>,
    ) -> Result<String, ErrorData> {
        let facts = self
            .engine()?
            .stat(Actor::Ai, &request.path)
            .await
            .map_err(refuse)?;

        serde_json::to_string(&serde_json::json!({
            "path": facts.path,
            "isDir": facts.is_dir,
            "size": facts.size,
            // **8 進数 4 桁の文字列**（`0644`）。数値で返すと先頭の 0 が落ちる。
            "permissions": facts.permissions,
            // UNIX 秒。**返してこないサーバーもある。**
            "modifiedUnixSeconds": facts.modified,
            "uid": facts.uid,
            "gid": facts.gid,
        }))
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

    // --- 許可リストのコマンド（D3） -----------------------------------------

    /// 人が許したコマンドの一覧。**サーバーへは触りません。**
    ///
    /// **これが無いと AI は当てずっぽうで呼びます。**呼んで断られた分は
    /// 記録に残り、人が「本当に要ったもの」を足す材料になります（D3 追記）。
    #[tool(
        description = "List the commands a human has allowed sshboard to run for you, with the \
                       exact text each one runs. This list is empty until a person fills it in - \
                       that is normal, not a fault. You cannot add to it yourself. Touches no \
                       remote server."
    )]
    pub async fn list_readonly_commands(&self) -> Result<String, ErrorData> {
        self.show("list_readonly_commands").await?;

        let listed = self.engine()?.readonly_commands().map_err(refuse)?;
        serde_json::to_string(&serde_json::json!({
            "commands": listed,
            // **空だったときに、AI が行き止まりにならないように。**
            "howToAdd": "A person adds entries to readonly.toml. Ask them; you cannot add any.",
        }))
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
    }

    /// 許可された 1 本を走らせる（D3）。
    ///
    /// **引数で任意の文字列をシェルへ渡す口ではありません。**
    /// 渡せるのは一覧の識別子だけで、走るのは人が書いた文字列そのものです。
    #[tool(
        description = "Run one of the commands a human listed in sshboard's allowlist, chosen by \
                       its id. You cannot pass a command line, arguments, or a shell string - only \
                       an id from list_readonly_commands. If the id is not on the list it is \
                       refused and the refusal is shown to the person and written down; ask them \
                       to add it rather than looking for another way in."
    )]
    pub async fn run_readonly(
        &self,
        Parameters(request): Parameters<ReadonlyCommandId>,
    ) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .run_readonly(Actor::Ai, &request.command_id)
            .await
            .map_err(refuse)?;

        let (out, out_cut) = capped(ran.out);
        let (err, err_cut) = capped(ran.err);
        serde_json::to_string(&serde_json::json!({
            "commandId": request.command_id,
            "stdout": out,
            // **空とは限らないし、空でも失敗しているとは限らない。**
            "stderr": err,
            // **返してこないサーバーもある。**分からないことを 0 と言わない。
            "exitCode": ran.status,
            "outputWasTruncated": out_cut || err_cut,
        }))
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
    }
}

#[tool_router(router = probe_tool_router, vis = "pub")]
impl SshboardMcp {
    /// 空き容量（D3・用途別）。
    #[tool(
        description = "Report free disk space on the connected server (df). Read-only: sshboard \
                       builds the command, you cannot alter it."
    )]
    pub async fn disk_usage(&self) -> Result<String, ErrorData> {
        let ran = self.engine()?.disk_usage(Actor::Ai).await.map_err(refuse)?;
        probe_json("disk_usage", ran)
    }

    /// プロセス一覧（D3・用途別）。
    #[tool(
        description = "List the processes running on the connected server (ps). Read-only: \
                       sshboard builds the command, you cannot alter it."
    )]
    pub async fn process_list(&self) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .process_list(Actor::Ai)
            .await
            .map_err(refuse)?;
        probe_json("process_list", ran)
    }

    /// listen しているポート（D3・用途別）。
    #[tool(
        description = "List the TCP ports the connected server is listening on. Falls back to \
                       netstat where ss is missing. Read-only: sshboard builds the command."
    )]
    pub async fn network_listen(&self) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .network_listen(Actor::Ai)
            .await
            .map_err(refuse)?;
        probe_json("network_listen", ran)
    }

    /// サービスの状態（D3・用途別）。
    #[tool(
        description = "Show the status of one systemd service on the connected server. You pass \
                       only the unit name - it is quoted, so it cannot become a second command. \
                       This reads the status; it does not start, stop or restart anything."
    )]
    pub async fn service_status(
        &self,
        Parameters(request): Parameters<ServiceName>,
    ) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .service_status(Actor::Ai, &request.service)
            .await
            .map_err(refuse)?;
        probe_json("service_status", ran)
    }

    /// ログの末尾（D3・用途別）。
    #[tool(
        description = "Read the last lines of one log file on the connected server (tail). The \
                       path is quoted, so it cannot become a command. This returns once - to \
                       keep watching a file, the person starts a follow on screen."
    )]
    pub async fn read_log(
        &self,
        Parameters(request): Parameters<ReadLog>,
    ) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .read_log(Actor::Ai, &request.path, request.lines.unwrap_or(200))
            .await
            .map_err(refuse)?;
        probe_json("read_log", ran)
    }
}

#[tool_router(router = search_tool_router, vis = "pub")]
impl SshboardMcp {
    /// 探す（D3・用途別）。
    #[tool(
        description = "Search the connected server for files by name, or for text inside files. \
                       You pass a directory to start from and a pattern - both are quoted, so \
                       neither can become a command. The search is bounded in depth and in how \
                       many hits come back, so a search from / will still return."
    )]
    pub async fn search(
        &self,
        Parameters(request): Parameters<Search>,
    ) -> Result<String, ErrorData> {
        let engine = self.engine()?;
        let hits = request.hits.unwrap_or(100);
        let ran = if request.in_contents.unwrap_or(false) {
            engine
                .search_content(Actor::Ai, &request.path, &request.pattern, hits)
                .await
        } else {
            engine
                .search_names(Actor::Ai, &request.path, &request.pattern, hits)
                .await
        }
        .map_err(refuse)?;

        probe_json("search", ran)
    }

    /// 何が入っていて、どの版か（D3・用途別）。
    #[tool(
        description = "Report the operating system of the connected server and the versions of \
                       the language runtimes it has. A runtime that is absent is simply left out \
                       - that is not a failure, so do not go looking for a cause."
    )]
    pub async fn runtime_versions(&self) -> Result<String, ErrorData> {
        let ran = self
            .engine()?
            .runtime_versions(Actor::Ai)
            .await
            .map_err(refuse)?;
        probe_json("runtime_versions", ran)
    }
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct Search {
    /// どこから探すか。サーバー上の絶対パス。
    pub path: String,
    /// 名前で探すときはワイルドカード（`*.conf`）、中身で探すときは語。
    pub pattern: String,
    /// `true` でファイルの中身を探す。**省略すると名前で探す。**
    #[serde(default)]
    pub in_contents: Option<bool>,
    /// 何件まで返すか。**省略すると 100 件。**
    #[serde(default)]
    pub hits: Option<u32>,
}

/// 走らせた結果を返す形。**stderr も終了コードも捨てません。**
fn probe_json(tool: &str, ran: sshboard_engine::Ran) -> Result<String, ErrorData> {
    let (out, out_cut) = capped(ran.out);
    let (err, err_cut) = capped(ran.err);
    serde_json::to_string(&serde_json::json!({
        "tool": tool,
        "stdout": out,
        // **空とは限らないし、空でも失敗しているとは限らない。**
        "stderr": err,
        // **返してこないサーバーもある。**分からないことを 0 と言わない。
        "exitCode": ran.status,
        "outputWasTruncated": out_cut || err_cut,
    }))
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ServiceName {
    /// systemd のユニット名（`nginx` / `postfix.service`）。
    pub service: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ReadLog {
    /// サーバー上の絶対パス。
    pub path: String,
    /// 末尾から何行返すか。**省略すると 200 行。**
    #[serde(default)]
    pub lines: Option<u32>,
}

/// 1 回の出力で返す上限。**AI の文脈を丸ごと食わせない。**
/// 実測でここに当たったら、分割して返す形を入れて上げる（YAGNI）。
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// 長い出力を切る。**切ったことを隠しません**（呼ぶ側が印を返します）。
fn capped(text: String) -> (String, bool) {
    if text.len() <= MAX_OUTPUT_BYTES {
        return (text, false);
    }

    // 文字の途中で切らない。**壊れた文字を返すと、原因の分からない化けになる。**
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ReadonlyCommandId {
    /// `list_readonly_commands` が返す識別子。**コマンドそのものではありません。**
    pub command_id: String,
}

#[cfg(test)]
mod tests {
    use super::{capped, MAX_OUTPUT_BYTES};

    #[test]
    fn short_output_comes_back_whole() {
        let (text, cut) = capped("uptime".to_string());

        assert_eq!(text, "uptime");
        assert!(!cut);
    }

    #[test]
    fn long_output_is_cut_and_says_so() {
        let (text, cut) = capped("a".repeat(MAX_OUTPUT_BYTES + 10));

        assert_eq!(text.len(), MAX_OUTPUT_BYTES);
        assert!(cut, "切ったことを伝えていない");
    }

    #[test]
    fn cutting_never_splits_a_character_in_half() {
        // **化けた文字を返すと、原因が出力なのか通信なのか分からなくなる。**
        let (text, cut) = capped("あ".repeat(MAX_OUTPUT_BYTES));

        assert!(cut);
        assert!(text.len() <= MAX_OUTPUT_BYTES);
        assert!(text.chars().all(|c| c == 'あ'), "文字が壊れている");
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
pub struct MaybeConnectionId {
    /// 省略すると、いま操作の宛先になっているもの。
    #[serde(default)]
    pub connection_id: Option<String>,
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

// --- 端末（D29） -------------------------------------------------------------

/// 一度に打てる上限。**貼り付け事故を小さくする。**
const MAX_TYPED: usize = 4096;

/// `console_open` の引数。
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct OpenConsole {
    /// 桁数。**省略すると 80。**
    pub cols: Option<u32>,
    /// 行数。**省略すると 24。**
    pub rows: Option<u32>,
}

/// `console_type` の引数。
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TypeIntoConsole {
    /// 打ち込む文字列。**改行を入れないと実行されません**（本物の端末と同じ）。
    pub text: String,
}

#[tool_router(router = console_tool_router, vis = "pub")]
impl SshboardMcp {
    /// 端末を握る（D29）。
    #[tool(
        description = "Take hold of the interactive console on the connected server and open a \
                       shell. sshboard shares one console between you and the person at the \
                       screen, and only one side may type at a time - while you hold it, their \
                       typing is locked, and they see on screen that you are holding it. \
                       They can take it back at any moment, and their Stop always works. If they \
                       do, your next keystroke is refused: that is them intervening, not a fault. \
                       Say so and ask, rather than trying to take it back."
    )]
    pub async fn console_open(
        &self,
        Parameters(request): Parameters<OpenConsole>,
    ) -> Result<String, ErrorData> {
        let cols = request.cols.unwrap_or(80).clamp(20, 500);
        let rows = request.rows.unwrap_or(24).clamp(5, 200);
        self.engine()?
            .console_open(Actor::Ai, cols, rows)
            .await
            .map_err(refuse)?;
        // **どの接続の端末かを必ず添える**（D25）。
        // 添えないと、タブを移したあとに「どこへ打っているのか」が分からなくなる。
        let on = self
            .engine()?
            .console_connection()
            .await
            .unwrap_or_else(|| "?".to_string());
        Ok(format!(
            "console opened on {on} ({cols}x{rows}). Read the output with read_stream. \
             It stays on {on} even if the focus moves to another connection."
        ))
    }

    /// 打ち込む。**握っているときだけ。**
    #[tool(
        description = "Type into the console you are holding. The text goes to the shell exactly \
                       as given - include a newline to run it. Read what came back with \
                       read_stream. Refused if the person has taken the console back."
    )]
    pub async fn console_type(
        &self,
        Parameters(request): Parameters<TypeIntoConsole>,
    ) -> Result<String, ErrorData> {
        let bytes = request.text.as_bytes();
        if bytes.len() > MAX_TYPED {
            return Err(ErrorData::invalid_params(
                format!("一度に打てるのは {MAX_TYPED} バイトまでです"),
                None,
            ));
        }
        self.engine()?
            .console_type(Actor::Ai, bytes)
            .await
            .map_err(refuse)?;
        Ok(format!(
            "typed {} bytes. Read the output with read_stream.",
            bytes.len()
        ))
    }

    /// 手を離す（D29）。**握ったまま離さない、を作らない。**
    #[tool(
        description = "Let go of the console and close the shell. Do this when you are done, so \
                       the person is not left locked out. Never fails."
    )]
    pub async fn console_stop(&self) -> Result<String, ErrorData> {
        self.engine()?.console_stop().await;
        Ok("console released".to_string())
    }
}
