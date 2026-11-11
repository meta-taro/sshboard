//! **サーバーに触る道具は、どの接続かを毎回名指しする**（2026-11-11・オーナーが案 A を選んだ）。
//!
//! 宛先は sshboard 全体で 1 つでした。**人が画面でタブを押しても動きます。**
//! 2026-11-11、ある席が測定機へ繋いだ直後に出した `read_file` / `list_directory` が、
//! 画面で押された別の本番機へ黙って届きました（読めたものは無かった）。
//!
//! だから AI の道具は**宛先に頼らず、`connection_id` で名指しする**。省いたら断る。
//!
//! **サーバーが無い環境でも走ります**（product-baseline §4）。建てるには `sh tools/test-server/up.sh`。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_diag::Diagnostics;
use sshboard_engine::Engine;
use sshboard_mcp::{serve, McpEndpoint, ServeParts};
use sshboard_ssh::{Auth, SshError, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 2222;
const USER: &str = "probe";
const TOKEN: &str = "test-token-not-a-real-secret";

/// 2 本とも同じテスト用サーバーへ繋ぎ、**書き込みの囲いだけ変えます。**
/// どちらへ届いたかは、囲いで断られるかどうかで分かります。
const ONE_ROOT: &str = "/home/probe/upload/target-one";
const TWO_ROOT: &str = "/home/probe/upload/target-two";

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

/// テスト用サーバーの指紋を調べる。
async fn fingerprint() -> String {
    let target = Target {
        id: Some("probe".into()),
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

/// 画面役。**帯の行に全部応えます**（無いと、どの道具も通らない・D16）。
fn fake_screen(band: &Band) {
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
    /// **人の役**。タブを押すのは人なので、ここから `focus` を呼びます。
    engine: Arc<Engine>,
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

    async fn call(&self, name: &str, arguments: serde_json::Value) -> String {
        self.post(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            })
            .to_string(),
        )
        .await
    }
}

fn connection_entry(id: &str, fingerprint: &str, root: &str) -> String {
    format!(
        "[[connections]]\nid = \"{id}\"\nname = \"{id}\"\n\
         host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\n\
         fingerprint = \"{fingerprint}\"\nwrite_roots = [\"{root}\"]\n\n"
    )
}

/// 「one」と「two」を開き、**人が one のタブを押した**状態にして返す。
async fn two_open_with_one_focused() -> Harness {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("connections.toml");
    let seen = fingerprint().await;
    std::fs::write(
        &path,
        format!(
            "version = 1\n\n{}{}",
            connection_entry("one", &seen, ONE_ROOT),
            connection_entry("two", &seen, TWO_ROOT)
        ),
    )
    .expect("接続一覧を書けない");

    let band = Band::new();
    fake_screen(&band);
    let engine = Arc::new(Engine::new(
        band.clone(),
        Arc::new(OutputStream::new()),
        path,
    ));
    let endpoint = serve(ServeParts {
        band,
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: Some(Arc::clone(&engine)),
        view: None,
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
        engine,
        _dir: dir,
    };
    harness.post(INITIALIZED.to_string()).await;

    for id in ["one", "two"] {
        let opened = harness
            .call("connect", serde_json::json!({ "connection_id": id }))
            .await;
        assert!(opened.contains("SHA256:"), "{id} を開けない: {opened}");
    }
    // **人がタブを押した。**宛先は one へ動きます。
    harness
        .engine
        .focus("one")
        .await
        .expect("one のタブを押せない");
    harness
}

#[tokio::test]
async fn a_tool_goes_to_the_connection_it_names_not_the_one_on_screen() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    // Arrange
    let harness = two_open_with_one_focused().await;

    // Act —— 画面は one。**名指しは two。**two の囲いの中へ置き場所を作る。
    let made = harness
        .call(
            "make_directory",
            serde_json::json!({ "connection_id": "two", "path": format!("{TWO_ROOT}/named") }),
        )
        .await;

    // Assert —— one へ行けば、two の囲いは one に無いので断られます。
    assert!(
        made.contains("ready"),
        "名指しした two ではなく、画面の one へ行った: {made}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn a_tool_that_does_not_name_its_connection_is_refused() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    // Arrange
    let harness = two_open_with_one_focused().await;

    // Act —— 名指しを省く。**画面の one へ黙って行かせない。**
    let listed = harness
        .call(
            "list_directory",
            serde_json::json!({ "path": "/home/probe" }),
        )
        .await;

    // Assert
    assert!(
        listed.contains("connection_id"),
        "名指しを省いたのに断らず、どこかへ届いた: {listed}"
    );
    assert!(
        listed.contains("\"error\"") || listed.contains("\"isError\":true"),
        "断っていない: {listed}"
    );

    harness.endpoint.shutdown();
}
