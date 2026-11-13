//! **ファイルのタブを、指定した接続・指定したフォルダで開く**（2026-11-13・PdM の依頼 #2035）。
//!
//! 中継のサーバーの設定フォルダへ人にファイルを置いてもらう場面で、AI にできたのは
//! `show_view files` でタブを切り替えるところまででした。**どのフォルダかを渡す口が無く**、
//! 人が上から手でたどることになりました。
//!
//! だから `show_view` に `connection_id` / `path` / `select` を足す。
//! 開けなかったときは、**理由ごとに別の答え**を返す（人へ何を頼むかが変わるため）。
//!
//! - `not_found` … そのフォルダが無い
//! - `denied` … そのフォルダを読む権限が無い
//! - `file_not_found` … フォルダはあるが、選ぶファイルが無い
//! - `busy` … 人が入力中・質問に答えている最中（画面は動かない）
//! - `no_screen` … 画面が無い
//!
//! **フォルダが在るかは、画面を動かす前に確かめます。**画面だけ動いて空の一覧、を作らない。
//!
//! **サーバーが無い環境でも走ります**（product-baseline §4）。建てるには `sh tools/test-server/up.sh`。

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_connections::ConnectionsWatch;
use sshboard_diag::Diagnostics;
use sshboard_engine::Engine;
use sshboard_mcp::{serve, ActOutcome, AgentAct, McpEndpoint, ServeParts, ShowView};
use sshboard_ssh::{Auth, SshError, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 2222;
const USER: &str = "probe";
const TOKEN: &str = "test-token-not-a-real-secret";

/// 在るフォルダと、その中に在る物。
const HOME: &str = "/home/probe";
const IN_HOME: &str = "upload";
/// 利用者 probe には読めないフォルダ。
const LOCKED: &str = "/root";

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

async fn fingerprint() -> String {
    let target = Target {
        id: Some("one".into()),
        host: HOST.into(),
        port: PORT,
        user: USER.into(),
        pinned_fingerprint: None,
        known_hosts: String::new(),
        write_scope: WriteScope::default(),
    };
    match SshSession::connect(&target, &Auth::Agent, Band::new(), &Diagnostics::new()).await {
        Err(SshError::UntrustedHost { seen, .. }) => seen.fingerprint,
        Ok(_) => panic!("初見のホストを通している"),
        Err(other) => panic!("繋げません: {other}"),
    }
}

/// 偽の画面。**届いた指示を控え、決めておいた答えを返します。**
struct FakeScreen {
    answer: ActOutcome,
    got: Mutex<Vec<AgentAct>>,
}

impl FakeScreen {
    fn answering(answer: ActOutcome) -> Arc<Self> {
        Arc::new(Self {
            answer,
            got: Mutex::new(Vec::new()),
        })
    }

    fn got(&self) -> Vec<AgentAct> {
        self.got.lock().expect("控えが壊れた").clone()
    }
}

impl ShowView for FakeScreen {
    fn show<'a>(
        &'a self,
        _view: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn act<'a>(
        &'a self,
        act: &'a AgentAct,
    ) -> Pin<Box<dyn Future<Output = Result<ActOutcome, String>> + Send + 'a>> {
        Box::pin(async move {
            self.got.lock().expect("控えが壊れた").push(act.clone());
            Ok(self.answer)
        })
    }
}

fn ack_everything(band: &Band) {
    let mut subscriber = band.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            event.ack();
        }
    });
}

struct Harness {
    endpoint: McpEndpoint,
    client: reqwest::Client,
    session: Option<String>,
    _dir: tempfile::TempDir,
}

impl Harness {
    async fn post(&self, body: String) -> String {
        let mut request = self
            .client
            .post(self.endpoint.url())
            .header("Authorization", format!("Bearer {TOKEN}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(body);
        if let Some(id) = &self.session {
            request = request.header("Mcp-Session-Id", id);
        }
        request
            .send()
            .await
            .expect("MCP へ届かない")
            .text()
            .await
            .expect("応答が読めない")
    }

    async fn show_view(&self, arguments: serde_json::Value) -> String {
        self.post(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": { "name": "show_view", "arguments": arguments }
            })
            .to_string(),
        )
        .await
    }
}

/// 接続「one」を開いた状態で返す。**`with_server` が偽なら、サーバーへは繋がない。**
async fn harness(view: Option<Arc<dyn ShowView>>, with_server: bool) -> Harness {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("connections.toml");
    let entry = if with_server {
        format!(
            "[[connections]]\nid = \"one\"\nname = \"one\"\nhost = \"{HOST}\"\nport = {PORT}\n\
             user = \"{USER}\"\nfingerprint = \"{}\"\n",
            fingerprint().await
        )
    } else {
        String::new()
    };
    std::fs::write(&path, format!("version = 1\n\n{entry}")).expect("接続一覧を書けない");

    let band = Band::new();
    ack_everything(&band);
    let engine = Arc::new(Engine::new(
        band.clone(),
        Arc::new(OutputStream::new()),
        path,
    ));
    if with_server {
        engine
            .connect(Actor::Human, "one", None)
            .await
            .expect("one を開けない");
    }
    let endpoint = serve(ServeParts {
        band,
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: Some(engine),
        view,
        capture: None,
        token: Some(TOKEN.to_string()),
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");

    let client = reqwest::Client::new();
    let init = client
        .post(endpoint.url())
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(INIT)
        .send()
        .await
        .expect("MCP へ届かない");
    let session = init
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let _ = init.text().await;
    let harness = Harness {
        endpoint,
        client,
        session,
        _dir: dir,
    };
    harness.post(INITIALIZED.to_string()).await;
    harness
}

macro_rules! needs_server {
    () => {
        if !server_is_up().await {
            println!("テスト用サーバーが建っていません（想定内・飛ばします）");
            return;
        }
    };
}

#[tokio::test]
async fn opens_the_folder_and_selects_the_file_on_the_named_connection() {
    needs_server!();
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": HOME, "select": IN_HOME
        }))
        .await;

    // Assert
    assert!(!reply.contains("\"isError\":true"), "{reply}");
    assert!(reply.contains("done"), "{reply}");
    assert_eq!(
        screen.got(),
        vec![AgentAct::Open {
            connection_id: "one".into(),
            path: Some(HOME.into()),
            select: Some(IN_HOME.into()),
        }]
    );
}

#[tokio::test]
async fn a_folder_that_does_not_exist_is_not_found_and_the_screen_does_not_move() {
    needs_server!();
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": "/home/probe/no-such-folder"
        }))
        .await;

    // Assert
    assert!(reply.contains("not_found"), "{reply}");
    assert!(screen.got().is_empty(), "無いフォルダで画面を動かした");
}

#[tokio::test]
async fn a_folder_it_may_not_read_is_denied_not_missing() {
    needs_server!();
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": LOCKED
        }))
        .await;

    // Assert
    assert!(reply.contains("denied"), "{reply}");
    assert!(!reply.contains("not_found"), "{reply}");
    assert!(screen.got().is_empty(), "読めないフォルダで画面を動かした");
}

#[tokio::test]
async fn a_file_to_select_that_is_not_in_the_folder_is_file_not_found() {
    needs_server!();
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": HOME, "select": "no-such-file.conf"
        }))
        .await;

    // Assert
    assert!(reply.contains("file_not_found"), "{reply}");
    assert!(screen.got().is_empty(), "無いファイルで画面を動かした");
}

#[tokio::test]
async fn a_person_who_is_busy_gets_busy_back() {
    needs_server!();
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Busy);
    let mcp = harness(Some(screen), true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": HOME
        }))
        .await;

    // Assert
    assert!(reply.contains("busy"), "{reply}");
}

#[tokio::test]
async fn without_a_screen_it_says_no_screen() {
    needs_server!();
    // Arrange
    let mcp = harness(None, true).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": HOME
        }))
        .await;

    // Assert
    assert!(reply.contains("no_screen"), "{reply}");
}

#[tokio::test]
async fn a_folder_is_only_for_the_files_tab() {
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), false).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({ "view": "console", "path": HOME }))
        .await;

    // Assert
    assert!(reply.contains("only for the files tab"), "{reply}");
    assert!(screen.got().is_empty());
}

#[tokio::test]
async fn select_is_a_name_in_the_folder_not_a_path() {
    // Arrange
    let screen = FakeScreen::answering(ActOutcome::Done);
    let mcp = harness(Some(screen.clone()), false).await;

    // Act
    let reply = mcp
        .show_view(serde_json::json!({
            "view": "files", "connection_id": "one", "path": HOME, "select": "../etc/passwd"
        }))
        .await;

    // Assert
    assert!(reply.contains("one name inside the folder"), "{reply}");
    assert!(screen.got().is_empty());
}
