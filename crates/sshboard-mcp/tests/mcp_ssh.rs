//! **AI が実際にファイルを上げるところまでを、外部クライアントと同じ経路で確かめる。**
//!
//! MCP（HTTP・合言葉つき）→ Engine → SSH 1 本 → sftp。
//! 途中に近道を作っていないことを、ここで固定します（PRD §4-1）。
//!
//! **サーバーが無い環境でも走ります**（product-baseline §4）。
//! 建てるには `sh tools/test-server/up.sh`。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
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

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

/// テスト用サーバーの指紋。**1 回だけ調べて使い回す**（sshd の MaxStartups 対策）。
static FINGERPRINT: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

async fn fingerprint() -> &'static String {
    FINGERPRINT
        .get_or_init(|| async {
            let target = Target {
                id: Some("local".into()),
                host: HOST.into(),
                port: PORT,
                user: USER.into(),
                pinned_fingerprint: None,
                known_hosts: String::new(),
                write_scope: WriteScope::default(),
            };
            match SshSession::connect(&target, &Auth::Agent, Band::new(), &Diagnostics::new()).await
            {
                Err(SshError::UntrustedHost { seen, .. }) => seen.fingerprint,
                Ok(_) => panic!("初見のホストを通している"),
                Err(other) => panic!("繋げません: {other}"),
            }
        })
        .await
}

/// 帯の受け取りを返し続ける画面役。**これが無いと、どのツールも通らない**（D16）。
///
/// 見えた行はその場で溜める。**終わるのを待たない**（MCP を止めても帯は生きている）。
fn fake_screen(band: &Band) -> Arc<std::sync::Mutex<Vec<String>>> {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut subscriber = band.subscribe();
    let collected = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            if let Ok(mut held) = collected.lock() {
                held.push(event.line().render());
            }
            event.ack();
        }
    });
    seen
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

    /// ツールを 1 本呼ぶ。**外部クライアントと同じ生の JSON-RPC。**
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

/// 合言葉つきの MCP を立て、初期化まで済ませて返す。
async fn harness(band: Band, write_roots: &[&str]) -> Harness {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("connections.toml");
    let roots = write_roots
        .iter()
        .map(|root| format!("\"{root}\""))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        &path,
        format!(
            "version = 1\n\n[[connections]]\nid = \"local\"\nname = \"Local test server\"\n\
             host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\n\
             fingerprint = \"{}\"\nwrite_roots = [{roots}]\n",
            fingerprint().await
        ),
    )
    .expect("接続一覧を書けない");

    let engine = Arc::new(Engine::new(
        band.clone(),
        Arc::new(OutputStream::new()),
        path,
    ));

    let endpoint = serve(ServeParts {
        band,
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: Some(engine),
        capture: // 画面は無い（ヘッドレス）。**`capture_window` は正直に断るだけ。**
        None,
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
    let session = init.headers().get("mcp-session-id").map(|value| {
        value
            .to_str()
            .expect("session id が UTF-8 でない")
            .to_owned()
    });
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

#[tokio::test]
async fn an_agent_connects_lists_and_uploads_through_one_ssh_session() {
    // **これが D22 の通しの確認。**外の AI から見えるのはこの経路だけ。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let screen = fake_screen(&band);
    let harness = harness(band, &["/home/probe/upload"]).await;

    // 1. 繋ぐ
    let opened = harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;
    assert!(opened.contains("SHA256:"), "指紋が返っていない: {opened}");
    // **ホストは渡さない**（CLAUDE.md 禁止事項 5）。
    // 書き込み許可のパスは**人が選んだもの**で、そこに利用者名が入ることはある
    // （`/home/<user>/...`）。それは渡してよい。渡さないのは接続先そのもの。
    assert!(!opened.contains(HOST), "接続先が AI へ漏れている: {opened}");

    // 2. 囲いの中へ置き場所を作る
    let made = harness
        .call(
            "make_directory",
            serde_json::json!({ "path": "/home/probe/upload/release" }),
        )
        .await;
    assert!(made.contains("ready"), "作れない: {made}");

    // 3. 中身を書いて上げる
    let wrote = harness
        .call(
            "write_file",
            serde_json::json!({
                "remote_path": "/home/probe/upload/release/via-mcp.txt",
                "content": "sshboard"
            }),
        )
        .await;
    assert!(wrote.contains("wrote 8 bytes"), "上げられない: {wrote}");

    // 4. **本当に届いているか**をサーバー側で確かめる
    let listed = harness
        .call(
            "list_directory",
            serde_json::json!({ "path": "/home/probe/upload/release" }),
        )
        .await;
    assert!(listed.contains("via-mcp.txt"), "サーバーに無い: {listed}");

    let read = harness
        .call(
            "read_file",
            serde_json::json!({ "path": "/home/probe/upload/release/via-mcp.txt" }),
        )
        .await;
    assert!(read.contains("sshboard"), "読み戻せない: {read}");

    // 5. すべて帯に出ている（PRD §4-2）
    harness.endpoint.shutdown();
    let lines = screen.lock().expect("画面役の記録を読めない").clone();
    assert!(
        lines.iter().any(|line| line.contains("via-mcp.txt")),
        "書き込みが帯に出ていない: {lines:?}"
    );
    assert!(
        lines.iter().all(|line| line.starts_with("[AI]")),
        "AI の操作でない行が混ざっている: {lines:?}"
    );
}

#[tokio::test]
async fn an_agent_cannot_write_outside_the_directories_a_human_allowed() {
    // **囲いの外は、サーバーへ届く前に断る**（D22）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &["/home/probe/upload"]).await;

    harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;

    let refused = harness
        .call(
            "write_file",
            serde_json::json!({
                "remote_path": "/home/probe/outside-via-mcp.txt",
                "content": "nope"
            }),
        )
        .await;
    assert!(
        refused.contains("書き込み") || refused.contains("error"),
        "囲いの外へ書けている: {refused}"
    );

    // **本当に届いていないこと**をサーバー側で確かめる。
    let listed = harness
        .call(
            "list_directory",
            serde_json::json!({ "path": "/home/probe" }),
        )
        .await;
    assert!(
        !listed.contains("outside-via-mcp.txt"),
        "断ったはずのファイルがサーバーにある: {listed}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn an_agent_that_has_not_connected_is_told_to_ask_a_human() {
    // 「駄目でした」で終わらせない（product-baseline §17）。
    // **次に何をすべきかが AI に分かる形で返す。**
    //
    // 繋がないテストだが、**足場（接続一覧）を作るのに指紋が要る**ので、
    // ここもサーバーが要る。**CI で実際に落ちた。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &[]).await;

    let answer = harness
        .call("list_directory", serde_json::json!({ "path": "/" }))
        .await;

    assert!(
        answer.contains("繋がっていません") || answer.contains("接続を開いて"),
        "何をすべきか分からない断り方: {answer}"
    );

    // 一覧にも接続先は出さない。**識別子と名前だけ**（CLAUDE.md 禁止事項 5）。
    let listed = harness
        .call("list_connections", serde_json::json!({}))
        .await;
    assert!(listed.contains("local"), "登録が見えない: {listed}");
    assert!(
        !listed.contains(HOST) && !listed.contains(USER),
        "接続先が AI へ漏れている: {listed}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn the_band_shows_who_did_it_even_when_the_ai_drives_everything() {
    // PRD §4-2。**人の行と AI の行が、同じ 1 本に並ぶこと。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let mut watching = band.subscribe();
    let harness = harness(band.clone(), &[]).await;

    // 人の側の 1 行も混ぜる。
    let human = band.record(Actor::Human, "cd /var/www");
    let _ = human.wait_acked(Duration::from_millis(50)).await;

    let calling =
        tokio::spawn(async move { harness.call("session_status", serde_json::json!({})).await });

    let mut rendered = Vec::new();
    for _ in 0..2 {
        let event = tokio::time::timeout(Duration::from_secs(5), watching.recv())
            .await
            .expect("帯へ来ない")
            .expect("帯が閉じている");
        rendered.push(event.line().render());
        event.ack();
    }
    calling.await.expect("パニック");

    assert!(
        rendered.iter().any(|line| line.starts_with("[Human]")),
        "人の行が無い: {rendered:?}"
    );
    assert!(
        rendered.iter().any(|line| line.starts_with("[AI]")),
        "AI の行が無い: {rendered:?}"
    );
}
