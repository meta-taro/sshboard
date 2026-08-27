//! アプリに同居する MCP サーバー本体（decisions D8）。
//!
//! **ツールは帯へ載せてからでないと応答を返さない。**
//! 返してから画面が追いつく形にすると、AI は人より先に動けてしまう。
//! それは PRD §4-2 の「誰が触ったかが画面に出る」を満たさない。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Deserialize;
use sshboard_band::{Actor, Band, DeliveryOutcome};
use sshboard_connections::{ConnectionEntry, ConnectionSummary, Connections, ConnectionsWatch};
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
    /// 接続一覧の置き場所。`None` なら OS の既定の場所。
    connections_path: Option<PathBuf>,
    /// 一覧が変わったことを画面へ押し出す口。
    /// **無ければ通知しないだけ**（ヘッドレスのテストではそれでよい）。
    connections_watch: Option<Arc<ConnectionsWatch>>,
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
            connections_path: None,
            connections_watch: None,
            ack_timeout,
            tool_router: Self::tool_router(),
        }
    }

    /// 接続一覧の置き場所を差し替える。**テストで OS の設定を汚さないため。**
    pub fn with_connections(mut self, path: PathBuf) -> Self {
        self.connections_path = Some(path);
        self
    }

    /// 一覧が変わったことを押し出す口を渡す。
    ///
    /// **これが無いと、AI が足した接続を人が知らないままになる**（PRD §4-2）。
    pub fn with_connections_watch(mut self, watch: Arc<ConnectionsWatch>) -> Self {
        self.connections_watch = Some(watch);
        self
    }

    /// 登録された接続の**識別子と名前だけ**を取り出す。
    ///
    /// **ここにホスト名を混ぜないこと**（CLAUDE.md 禁止事項 5）。
    /// 混ぜてよいかどうかは `ConnectionSummary` の側で守っている。
    pub fn connection_summaries(&self) -> Result<Vec<ConnectionSummary>, ErrorData> {
        let path = self.connections_file()?;

        Connections::load_or_empty(&path)
            .map(|connections| connections.summaries())
            .map_err(|error| {
                // 中身を載せない。**接続先が混ざりうる**（PRD §8）。
                ErrorData::internal_error(format!("cannot read connections: {error}"), None)
            })
    }

    /// 接続一覧のファイルの場所。
    fn connections_file(&self) -> Result<PathBuf, ErrorData> {
        match &self.connections_path {
            Some(path) => Ok(path.clone()),
            None => sshboard_connections::default_path().map_err(|error| {
                ErrorData::internal_error(format!("cannot locate connections: {error}"), None)
            }),
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

    /// 接続を 1 件登録する。**ローカルの設定ファイルにだけ書きます。**
    ///
    /// **サーバーへは 1 バイトも書きません**（D2 / D21）。
    /// 秘密は受け取りません。パスフレーズは ssh-agent か OS ストアにあります（D11）。
    #[tool(
        description = "Register one connection in sshboard's local list. Writes only to the local config file - never to any remote server. Accepts no passwords or passphrases."
    )]
    pub async fn register_connection(
        &self,
        Parameters(request): Parameters<RegisterConnection>,
    ) -> Result<String, ErrorData> {
        // 弾くものは帯へ載せる前に弾く。**書けない要求で帯を埋めない。**
        check_id(&request.id)?;
        check_present("host", &request.host)?;
        check_present("user", &request.user)?;

        // **識別子だけを帯へ載せる。**帯は画面に出るので、
        // ホスト名を載せると画面の写真に接続先が写る（PRD §8）。
        self.show(&format!("register_connection {}", request.id))
            .await?;

        let path = self.connections_file()?;
        let held = Connections::load_or_empty(&path).map_err(|error| {
            ErrorData::internal_error(format!("cannot read connections: {error}"), None)
        })?;

        // **黙って上書きしない。**人が登録したものが消える。
        if held.get(&request.id).is_some() {
            return Err(ErrorData::invalid_params(
                format!("connection `{}` already exists", request.id),
                None,
            ));
        }

        let entry = ConnectionEntry {
            id: request.id.clone(),
            name: request.name,
            host: request.host,
            port: request.port,
            user: request.user,
            // 空文字は「指定なし」として扱う。ssh-agent を使う（D11）。
            key_path: request.key_path.filter(|path| !path.trim().is_empty()),
            keyring_passphrase_ref: None,
            fingerprint: None,
            known_hosts: None,
        };

        let next = Connections {
            version: held.version,
            connections: held
                .connections
                .into_iter()
                .chain(std::iter::once(entry))
                .collect(),
        };
        next.save(&path).map_err(|error| {
            ErrorData::internal_error(format!("cannot save connections: {error}"), None)
        })?;

        // **画面へ押し出す。**ファイルに入っただけでは、人は知らないまま。
        if let Some(watch) = &self.connections_watch {
            watch.notify();
        }

        Ok(format!("registered `{}`", request.id))
    }

    /// 登録された接続の一覧。
    ///
    /// **識別子と名前だけを返します。**ホスト名・IP・利用者名・鍵のパス・
    /// 認証情報は 1 つも返しません（D11 / CLAUDE.md 禁止事項 5）。
    #[tool(
        description = "List the connections registered in sshboard. Returns identifiers and display names only - never hosts, users, or credentials."
    )]
    pub async fn list_connections(&self) -> Result<String, ErrorData> {
        self.show("list_connections").await?;

        let summaries = self.connection_summaries()?;
        serde_json::to_string(&summaries).map_err(|error| {
            ErrorData::internal_error(format!("cannot render connections: {error}"), None)
        })
    }
}

/// `register_connection` の引数。
///
/// **秘密は受け取りません。**パスフレーズもパスワードも項目にありません。
/// 鍵は ssh-agent か、鍵ファイルのパスだけです（D11）。
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct RegisterConnection {
    /// 機械が使う識別子。英数字と `.` `_` `-` のみ。
    pub id: String,
    /// 人が読む名前。
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    /// 秘密鍵のパス。**省略すると ssh-agent を使う**（推奨）。
    #[serde(default)]
    pub key_path: Option<String>,
}

fn default_ssh_port() -> u16 {
    22
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SshboardMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

/// 識別子はファイルにも帯にも出る。**変な文字を通さない。**
fn check_id(id: &str) -> Result<(), ErrorData> {
    let usable = !id.trim().is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));

    if usable {
        Ok(())
    } else {
        Err(ErrorData::invalid_params(
            "id must be non-empty and use only A-Z a-z 0-9 . _ -".to_string(),
            None,
        ))
    }
}

fn check_present(field: &str, value: &str) -> Result<(), ErrorData> {
    if value.trim().is_empty() {
        Err(ErrorData::invalid_params(
            format!("{field} must not be empty"),
            None,
        ))
    } else {
        Ok(())
    }
}
