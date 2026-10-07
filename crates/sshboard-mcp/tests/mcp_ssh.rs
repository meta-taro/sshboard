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
    /// **人の役**。端末の許可（D42）は人しか出せないので、テストでは
    /// ここを通して押します。**AI 側の口から許可は出せません。**
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
        engine: Some(Arc::clone(&engine)),
        view: None,
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
        engine,
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

#[tokio::test]
async fn an_agent_can_see_every_console_and_read_each_one_on_its_own() {
    // **これが D60 の要点です。**端末が増えても、
    // **AI が人と同じものを見られなければ、増やした意味がありません。**
    //
    // > 繋いだタブ、画面分割、どれも MCP でエージェントが見れるのが望ましい
    //
    // 見られるとは、**どれが開いているか**と**それぞれの中身**の両方です。
    // 一覧だけでは読めず、1 本分の出力だけでは「他に何が開いているか」が分かりません。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &[]).await;

    harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;

    // **まだ 1 本も開いていない。**空を「壊れている」と読ませない。
    let empty = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        empty.contains(r#"\"open\":[]"#),
        "端末が 0 本のときに空だと言えていない: {empty}"
    );

    // 人が開く。**AI が開いたのではありません。**
    harness
        .engine
        .console_open(Actor::Human, Some("local"), 80, 24)
        .await
        .expect("人が開けない");

    // **人が開いた端末が、AI から見える。**ここが「同じ視点」です。
    let listed = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        listed.contains(r#"\"connection\":\"local\""#),
        "人が開いた端末が AI から見えない: {listed}"
    );
    assert!(
        listed.contains(r#"\"holder\":\"human\""#),
        "誰が握っているかが見えない: {listed}"
    );

    // 人が打ったものも読める。
    harness
        .engine
        .console_type(Actor::Human, Some("local"), b"echo SEEN_BY_THE_AGENT\n")
        .await
        .expect("人が打てない");

    let mut seen = String::new();
    for _ in 0..50 {
        seen = harness
            .call(
                "read_console",
                serde_json::json!({ "connection_id": "local" }),
            )
            .await;
        if seen.contains("SEEN_BY_THE_AGENT") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        seen.contains("SEEN_BY_THE_AGENT"),
        "人が打った端末の中身が AI から読めない: {seen}"
    );

    // **繋がっていない接続を尋ねたら、正直に断る。**空を返さない。
    let nowhere = harness
        .call(
            "read_console",
            serde_json::json!({ "connection_id": "not-a-connection" }),
        )
        .await;
    assert!(
        nowhere.contains("not-a-connection"),
        "知らない接続を尋ねたのに、何が悪いか言っていない: {nowhere}"
    );

    // **`read_stream` も、宛先の出力を指したまま**（D60 で口を分けた取り落ち防止）。
    let stream = harness.call("read_stream", serde_json::json!({})).await;
    assert!(
        stream.contains("SEEN_BY_THE_AGENT"),
        "read_stream が宛先の出力を指していない: {stream}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn an_agent_is_told_when_its_console_request_was_already_allowed() {
    // **別のセッションからの報告**（2026-09-29・RC で届いた）——
    //
    // > 1回目 console_open → 「いま人に尋ねています」／人が許可
    // > 2回目 console_open → 同じエラー文／人がもう一度許可
    // > 3回目 console_open → 同じエラー文
    //
    // オーナーの言葉 ——「したって。わからないの？もうなんべんもしてる」
    //
    // `pending_status` が返していたのはこれで、**2 つの状態が区別できません。**
    //
    // ```json
    // {"console":{"holder":"human","waitingForTheHuman":false}}
    // ```
    //
    // (a) 人がいま打っている（待つべき）／(b) 許可は出た（呼べば取れる）
    //
    // **`waitingForPassphrase` には専用の枠があるのに、端末には無かった。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &[]).await;
    harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;

    // AI が頼む。**握れないのが正しい。**
    let asked = harness.call("console_open", serde_json::json!({})).await;
    assert!(
        asked.contains("人に尋ねています"),
        "許可なく開けてしまった: {asked}"
    );

    // **人の答え待ちだと言えている。**
    let waiting = harness.call("pending_status", serde_json::json!({})).await;
    assert!(
        waiting.contains(r#"\"granted\":false"#),
        "答え待ちなのに、そう言えていない: {waiting}"
    );

    // 人が許す（画面の役）。
    harness
        .engine
        .console_answer(Actor::Human, true)
        .await
        .expect("人が許せない");

    // **ここが報告の本体。**呼び直さずに「許可が出た」と分かる。
    let granted = harness.call("pending_status", serde_json::json!({})).await;
    assert!(
        granted.contains(r#"\"granted\":true"#),
        "**許可が出たのに、AI へ伝わっていない**（報告の形）: {granted}"
    );
    assert!(
        granted.contains(r#"\"youMayOpen\":\"local\""#),
        "どの接続を開けるのかが言えていない: {granted}"
    );
    assert!(
        granted.contains(r#"\"holder\":\"allowedToYou\""#),
        "**人が使っていると言い続けている**（これが「気づけない」の正体）: {granted}"
    );

    // そして 1 回で開ける。
    let opened = harness.call("console_open", serde_json::json!({})).await;
    assert!(
        opened.contains("console opened"),
        "許可が出たのに開けない: {opened}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn a_console_can_be_named_by_its_number() {
    // **同じサーバに 2 画面**（実運用の言葉・2026-10-02）——
    //
    // > 同じサーバに２画面入る場合もある。タブとか、画面分割できると尚良い
    //
    // 接続の名前だけで指していると、**2 本開いた日にどちらの話か言えません。**
    // 番号で指せる口を、画面分割より**先に**用意します ——
    // 後からだと「増えたが指せない」を一度作ることになります。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &[]).await;
    harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;
    harness
        .engine
        .console_open(Actor::Human, Some("local"), 80, 24)
        .await
        .expect("人が開けない");

    // **一覧に番号が出る。**
    let listed = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        listed.contains(r#"\"id\":1"#),
        "端末の番号が一覧に出ていない: {listed}"
    );

    harness
        .engine
        .console_type(Actor::Human, Some("local"), b"echo BY_NUMBER\n")
        .await
        .expect("人が打てない");

    // **番号で読める。**
    let mut seen = String::new();
    for _ in 0..50 {
        seen = harness
            .call("read_console", serde_json::json!({ "console_id": 1 }))
            .await;
        if seen.contains("BY_NUMBER") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(seen.contains("BY_NUMBER"), "番号で読めない: {seen}");

    // **無い番号は、正直に断る。**空を返さない。
    let nowhere = harness
        .call("read_console", serde_json::json!({ "console_id": 99 }))
        .await;
    assert!(
        nowhere.contains("no console #99"),
        "無い番号に何か返している: {nowhere}"
    );

    // **何も指さなければ、どう指すかを言う。**
    let unnamed = harness.call("read_console", serde_json::json!({})).await;
    assert!(
        unnamed.contains("console_id"),
        "指し方を教えていない: {unnamed}"
    );

    harness.endpoint.shutdown();
}

#[tokio::test]
async fn what_the_person_can_see_is_what_the_agent_is_told() {
    // **PRD §4-0 の「同じ視点」**。オーナーの言葉 ——
    //
    // > 私の思想は、私のこの視覚と、あなたと同化することです。
    // > **同じ視点で物事を見ることです。**
    //
    // これが無かった間、`personIsLookingAtIt` は**「宛先の接続かどうか」**で
    // 答えていました。同じサーバに 2 枚開くと**両方 true** になり、
    // タブに隠れた 3 枚目も true でした。**AI へ嘘が届いていました。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let _screen = fake_screen(&band);
    let harness = harness(band, &[]).await;
    harness
        .call("connect", serde_json::json!({ "connection_id": "local" }))
        .await;

    let first = harness
        .engine
        .console_open_new(Actor::Human, "local", 80, 24)
        .await
        .expect("1 枚目が開けない");
    let second = harness
        .engine
        .console_open_new(Actor::Human, "local", 80, 24)
        .await
        .expect("2 枚目が開けない");

    // **画面が何も言っていない段。**「見えていない」ではなく「分からない」。
    let unknown = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        unknown.contains(r#"\"personIsLookingAtIt\":null"#),
        "**画面が言っていないのに、見えている／いないを言い切った**: {unknown}"
    );
    assert!(
        unknown.contains(r#"\"onScreen\":null"#),
        "分からないことを null で言えていない: {unknown}"
    );

    // **画面が「1 枚目だけ見えていて、そこへ打っている」と言う。**
    harness
        .engine
        .report_on_screen(vec![first], Some(first))
        .await;

    let said = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        said.contains(&format!(r#"\"onScreen\":[{first}]"#)),
        "画面が言ったことが届いていない: {said}"
    );
    assert!(
        said.contains(&format!(r#"\"personIsTypingInto\":{first}"#)),
        "どこへ打っているかが届いていない: {said}"
    );

    // **2 枚目は「見えていない」と言えること。**ここが嘘だったところです。
    let rows: Vec<&str> = said.split("\\\"id\\\":").collect();
    let about_second = rows
        .iter()
        .find(|row| row.starts_with(&second.to_string()))
        .unwrap_or_else(|| panic!("2 枚目の行が無い: {said}"));
    assert!(
        about_second.contains(r#"\"personIsLookingAtIt\":false"#),
        "**見えていない面を「人が見ている」と言っている**: {about_second}"
    );

    // **端末の面を離れたら、1 枚も見えていない。**
    harness.engine.report_on_screen(vec![], None).await;
    let away = harness.call("list_consoles", serde_json::json!({})).await;
    assert!(
        away.contains(r#"\"onScreen\":[]"#),
        "人が別の面を見ていることを言えていない: {away}"
    );
    assert!(
        !away.contains(r#"\"personIsLookingAtIt\":true"#),
        "**誰も見ていないのに、見られていると言っている**: {away}"
    );

    harness.endpoint.shutdown();
}
