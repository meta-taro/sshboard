//! アプリに同居する MCP サーバー本体（decisions D8）。
//!
//! **ツールは帯へ載せてからでないと応答を返さない。**
//! 返してから画面が追いつく形にすると、AI は人より先に動けてしまう。
//! それは PRD §4-2 の「誰が触ったかが画面に出る」を満たさない。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Deserialize;
use sshboard_band::{Actor, Band, DeliveryOutcome};
use sshboard_connections::{ConnectionEntry, ConnectionSummary, Connections, ConnectionsWatch};
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;

/// 撮った画像の長辺の既定値。**dbboard と揃える**（同じ操作感にする）。
const DEFAULT_MAX_EDGE: u32 = 1400;

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
    /// サーバーへ触るための唯一の経路（PRD §4-1）。
    /// **無ければ SSH 系のツールが「繋げません」と正直に断るだけ**で、
    /// 帯・出力・接続一覧のツールはそのまま使える（ヘッドレスのテストがそれ）。
    engine: Option<Arc<Engine>>,
    /// 画面を撮る口（D26）。**無ければ「画面がありません」と正直に断る。**
    /// ヘッドレスのテストは、これが無いまま走る。
    capture: Option<Arc<dyn crate::capture::WindowCapture>>,
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
            engine: None,
            capture: None,
            ack_timeout,
            // 帯・出力・接続一覧の口と、サーバーへ触る口。**同じ 1 つのサーバーに載る。**
            tool_router: Self::tool_router()
                + Self::ssh_tool_router()
                + Self::console_tool_router()
                // 用途別の読み取り（D3）。**任意コマンドを作らずに済ませる方の半分。**
                + Self::probe_tool_router()
                + Self::search_tool_router(),
        }
    }

    /// サーバーへ触る経路を渡す。**これが無いと SSH 系のツールは動かない。**
    ///
    /// **接続一覧の置き場所も、ここで実行体に合わせる。**
    /// 別々に持たせると、`list_connections` が見ている一覧と `connect` が引く一覧が
    /// 食い違いうる。**実際にテストで食い違った。**
    pub fn with_engine(mut self, engine: Arc<Engine>) -> Self {
        self.connections_path = Some(engine.connections_path().to_path_buf());
        self.engine = Some(engine);
        self
    }

    /// サーバーへ触る経路。**無いことを黙って握り潰さない。**
    pub(crate) fn engine(&self) -> Result<&Arc<Engine>, ErrorData> {
        self.engine.as_ref().ok_or_else(|| {
            ErrorData::internal_error(
                "this sshboard build has no SSH engine attached".to_string(),
                None,
            )
        })
    }

    /// 接続一覧の置き場所を差し替える。**テストで OS の設定を汚さないため。**
    /// 画面を撮る口を差す（D26）。**Tauri を持っている側だけが差せる。**
    pub fn with_capture(mut self, capture: Arc<dyn crate::capture::WindowCapture>) -> Self {
        self.capture = Some(capture);
        self
    }

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
    pub(crate) async fn show(&self, text: &str) -> Result<(), ErrorData> {
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
            // **AI にパスワードを預けさせない**（D11 / §14）。
            // 秘密を投入するのは人だけです。
            keyring_password_ref: None,
            fingerprint: None,
            known_hosts: None,
            color: request.color.filter(|value| !value.trim().is_empty()),
            tag: request.tag.filter(|value| !value.trim().is_empty()),
            // **AI が登録した接続では、AI は書けない**（D22）。
            // 自分で登録して自分に許可を出せるなら、囲いは意味を持たない。
            // 書き込み許可を出すのは、画面を見ている人だけ。
            write_roots: Vec::new(),
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

    /// 既にある接続に**印（タグと色）だけ**を付ける。
    ///
    /// **ホストも利用者名も鍵も変えられません**（D21）。
    /// サーバーへは 1 バイトも書きません。
    #[tool(
        description = "Set the tag and colour mark on an existing connection. Changes only the mark - never the host, user, or key. Writes only to the local config file."
    )]
    pub async fn mark_connection(
        &self,
        Parameters(request): Parameters<MarkConnection>,
    ) -> Result<String, ErrorData> {
        check_id(&request.id)?;
        self.show(&format!("mark_connection {}", request.id))
            .await?;

        let path = self.connections_file()?;
        let held = Connections::load_or_empty(&path).map_err(|error| {
            ErrorData::internal_error(format!("cannot read connections: {error}"), None)
        })?;

        if held.get(&request.id).is_none() {
            return Err(ErrorData::invalid_params(
                format!("connection `{}` is not registered", request.id),
                None,
            ));
        }

        // **作り直して差し替える。**元の一覧を書き換えない。
        let marked: Vec<ConnectionEntry> = held
            .connections
            .into_iter()
            .map(|entry| {
                if entry.id != request.id {
                    return entry;
                }
                ConnectionEntry {
                    color: blank_to_none(request.color.clone()),
                    tag: blank_to_none(request.tag.clone()),
                    ..entry
                }
            })
            .collect();

        // 配色に無い色・長すぎるタグは、ここで save が弾く。
        Connections {
            version: held.version,
            connections: marked,
        }
        .save(&path)
        .map_err(|error| {
            ErrorData::invalid_params(format!("cannot save connections: {error}"), None)
        })?;

        if let Some(watch) = &self.connections_watch {
            watch.notify();
        }

        Ok(format!("marked `{}`", request.id))
    }

    /// 登録された接続の一覧。
    ///
    /// **識別子と名前だけを返します。**ホスト名・IP・利用者名・鍵のパス・
    /// 認証情報は 1 つも返しません（D11 / CLAUDE.md 禁止事項 5）。
    /// 画面を 1 枚撮る（D26）。**既定は伏せる。**
    ///
    /// 型検査は崩れを 1 件も止められなかった（1 日で 3 件出て、3 件とも人が見つけた）。
    /// **AI が自分で画面を見られないと、同じことが繰り返されます。**
    #[tool(
        description = "Photograph the sshboard window and return it as a PNG, so you can see \
                       what the app actually renders — a broken layout, a menu that fell back to \
                       English, text overflowing its box. Nothing here touches a remote server. \
                       \
                       By default the capture is redacted: connection names, tags, remote paths, \
                       fingerprints and file listings are painted over BEFORE the shot is taken, \
                       so an unredacted image is never produced. Sizes, positions, overlaps and \
                       overflow are all preserved, which is what you need to spot a broken layout. \
                       \
                       Ask for redact=false only when a human has told you to. The window belongs \
                       to the person sitting in front of it: describe what you see, and do not \
                       paste the image anywhere public. \
                       \
                       It fails when the sshboard window is not open, or when the operating system \
                       has not granted screen-recording permission. Neither is fixed by retrying — \
                       say so and ask a human."
    )]
    pub async fn capture_window(
        &self,
        Parameters(request): Parameters<CaptureWindow>,
    ) -> Result<CallToolResult, ErrorData> {
        let Some(capture) = self.capture.as_ref() else {
            return Err(ErrorData::internal_error(
                "画面がありません。sshboard のウィンドウが開いている必要があります".to_string(),
                None,
            ));
        };

        // **省略したら伏せる**（D26）。ここを `unwrap_or(false)` にした瞬間、
        // 引数を書き忘れた呼び出しが接続先を写します。
        let redact = request.redact.unwrap_or(true);
        let max_edge = request
            .max_edge
            .unwrap_or(DEFAULT_MAX_EDGE)
            .clamp(200, 4000);

        // **人の画面を撮ることも帯に出す**（PRD §4-2）。黙って撮らない。
        self.show(&format!(
            "capture_window（{}）",
            if redact {
                "伏せて撮る"
            } else {
                "**伏せずに撮る**"
            }
        ))
        .await?;

        let shot = capture
            .capture(redact, max_edge)
            .await
            .map_err(|why| ErrorData::internal_error(why, None))?;

        let told = format!(
            "{} / 実寸 {}x{} / 返した画像 {}x{} / {}",
            shot.title,
            shot.width,
            shot.height,
            shot.scaled_width,
            shot.scaled_height,
            if shot.redacted {
                "伏せて撮りました"
            } else {
                "**伏せずに撮りました**"
            }
        );
        let encoded = BASE64.encode(&shot.png);
        Ok(CallToolResult::success(vec![
            ContentBlock::text(told),
            ContentBlock::image(encoded, "image/png"),
        ]))
    }

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

/// `capture_window` の引数。**どちらも省略できます。**
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CaptureWindow {
    /// 伏せて撮るか。**省略したら伏せます**（D26）。
    ///
    /// 偽にしてよいのは、**人がそう言ったときだけ**です。
    pub redact: Option<bool>,
    /// 返す画像の長辺（画素）。**引き伸ばしはしません。**
    pub max_edge: Option<u32>,
}

/// `mark_connection` の引数。**印だけを変えます。**
///
/// ホストも利用者名も鍵も変えられません。**接続先を書き換える口にしない。**
#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct MarkConnection {
    /// 印を付ける接続の識別子。
    pub id: String,
    /// タグ。`prod` / `本番` / `開発2` など。**12 文字まで。**空文字で外す。
    #[serde(default)]
    pub tag: Option<String>,
    /// 色。`red` `orange` `yellow` `green` `teal` `blue` `purple` `pink` のどれか。
    /// **16 進数は受け付けません。**空文字で外す。
    #[serde(default)]
    pub color: Option<String>,
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
    /// 秘密鍵のパス。
    ///
    /// 省略すると ssh-agent を使います。**ただし Windows では既定で使えません** —
    /// `ssh-agent` サービスがスタートアップ **Disabled** で出荷されるためです
    /// （Windows 11 Home 26200 で実測・Issue #4）。
    /// **Windows では、鍵のパスを指定してください。**
    /// パスワードで繋ぐ場合は、画面（GUI）から登録します
    /// — **AI に秘密は預けさせません**（D11 / §14）。
    #[serde(default)]
    pub key_path: Option<String>,
    /// 印のタグ。`prod` / `本番` / `開発2` など。**12 文字まで。**
    #[serde(default)]
    pub tag: Option<String>,
    /// 印の色。`red` `orange` `yellow` `green` `teal` `blue` `purple` `pink` のどれか。
    /// **16 進数は受け付けません**（配色側が明暗を選ぶため）。
    #[serde(default)]
    pub color: Option<String>,
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

/// 空文字は「印なし」。**空文字をそのまま保存すると、空のタグが行に出る。**
fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|held| !held.trim().is_empty())
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
