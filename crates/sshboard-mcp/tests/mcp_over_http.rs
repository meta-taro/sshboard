//! Issue 001 の完了条件を、**外部クライアントと同じ経路**で確かめる。
//!
//! rmcp のクライアント SDK を使わず、生の JSON-RPC を HTTP へ投げる。
//! SDK を挟むと「SDK 同士が話せた」ことしか分からず、
//! 別実装の MCP クライアントで動く保証にならない。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_connections::ConnectionsWatch;
use sshboard_mcp::{serve, McpEndpoint, ServeParts};
use sshboard_stream::OutputStream;

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED_BODY: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const PING_BODY: &str =
    r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ping","arguments":{}}}"#;
const LIST_BODY: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}"#;

/// 画面の代わり。帯へ来た行を 1 本受けて ack し、その行を返す。
fn fake_screen(band: &Band) -> tokio::task::JoinHandle<String> {
    let mut subscriber = band.subscribe();
    tokio::spawn(async move {
        let event = subscriber.recv().await.expect("帯が閉じている");
        let rendered = event.line().render();
        assert_eq!(event.line().actor(), Actor::Ai);
        event.ack();
        rendered
    })
}

async fn post(
    client: &reqwest::Client,
    endpoint: &McpEndpoint,
    session: Option<&str>,
    body: &'static str,
) -> reqwest::Response {
    post_with(
        client,
        &endpoint.url(),
        session,
        body,
        Some(endpoint.token().to_string()),
    )
    .await
}

/// 合言葉を付けずに、あるいは違う合言葉で投げる口（D23 の確認用）。
async fn post_with(
    client: &reqwest::Client,
    url: &str,
    session: Option<&str>,
    body: &'static str,
    token: Option<String>,
) -> reqwest::Response {
    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(body);
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    if let Some(id) = session {
        request = request.header("Mcp-Session-Id", id);
    }
    request.send().await.expect("MCP へ届かない")
}

#[tokio::test]
async fn an_external_client_calling_ping_over_http_puts_a_line_on_the_band() {
    // Arrange
    let band = Band::new();
    let screen = fake_screen(&band);
    let endpoint = serve(ServeParts {
        band,
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    // Act
    let init = post(&client, &endpoint, None, INIT_BODY).await;
    assert_eq!(init.status(), 200, "initialize が通らない");
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().expect("session id が UTF-8 でない").to_owned());
    let _ = init.text().await;

    let ack = post(&client, &endpoint, session.as_deref(), INITIALIZED_BODY).await;
    assert!(
        ack.status().is_success(),
        "initialized が通らない: {}",
        ack.status()
    );

    let call = post(&client, &endpoint, session.as_deref(), PING_BODY).await;
    let status = call.status();
    let body = call.text().await.expect("応答が読めない");

    let line = tokio::time::timeout(Duration::from_secs(5), screen)
        .await
        .expect("帯へ行が来ない")
        .expect("画面役がパニックした");

    // Assert
    assert_eq!(status, 200, "tools/call が通らない: {body}");
    assert!(body.contains("pong"), "応答に pong が無い: {body}");
    assert!(line.starts_with("[AI]"), "行頭が [AI] でない: {line:?}");
    assert!(line.contains("ping"), "行に ping が無い: {line:?}");

    endpoint.shutdown();
}

#[tokio::test]
async fn the_server_advertises_only_the_phase_zero_tools() {
    // 任意コマンドの口を足していないことを、ここで機械的に見張る（decisions D3）。
    // Arrange
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    // Act
    let init = post(&client, &endpoint, None, INIT_BODY).await;
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_owned());
    let _ = init.text().await;
    post(&client, &endpoint, session.as_deref(), INITIALIZED_BODY).await;
    let listed = post(&client, &endpoint, session.as_deref(), LIST_BODY)
        .await
        .text()
        .await
        .unwrap();

    // Assert
    for expected in [
        "ping",
        "read_stream",
        "list_connections",
        "register_connection",
        // サーバーへ触る口（D22 以降）。**囲いつきの書き込みまでがここに載る。**
        "connect",
        "session_status",
        "list_directory",
        "upload_file",
        // **詰まったときに AI が自分で状況を掴む口。**
        "diagnostics",
        // **複数の接続をタブで持つ**（D25）。宛先を変える口。
        "focus_connection",
        // **直す口**（Issue #5）。`register_connection` は衝突したら断るので、
        // 直すのは別の口。**指紋・ホスト・利用者・ポートには触れない。**
        "update_connection",
        // **AI が自分で画面を見る口**（D26）。型検査は崩れを 1 件も止められなかった。
        "capture_window",
        // **人と AI が共有する端末**（D29 が D3 を条件つきで覆した）。
        // 投げっぱなしの `run_command` ではなく、**人が見ていて止められる対話コンソール**。
        "console_open",
        "console_type",
        "console_stop",
        // **AI が「こちらを見て」と言える口**（D44 / Issue #15）。
        // 承認も実行も増えません —— 奪えないことは画面側が担保します。
        "show_view",
        // **人が答えたかを知る口**（Issue #20）。帯にも画面にも何も出さないので、
        // **ポーリングしても害がありません** —— それが要点です。
        "pending_status",
        // **状態を変える操作**（D45 / Issue #17）。渡せるのは id だけ。
        "run_operation",
        "list_operations",
        // **この道具が何なのかを名乗る口**（実機の要望・2026-09-09）。
        // 初めて繋いだ AI が、何をしてよくて何が駄目かをここで読めます。
        "about_sshboard",
    ] {
        assert!(
            listed.contains(expected),
            "{expected} が一覧に無い: {listed}"
        );
    }

    // **本数を数える。**
    //
    // 上の一覧は「在ること」しか見ないので、**足しても誰も気づきません。**
    // 記録の表（`.claude/project-status.md` / `CLAUDE.md`）と実物は、
    // **これまで 3 回ずれました**（16→15・29→30・30→34・34→35）。毎回、
    // **足したときに表を直していない**のが原因です。
    //
    // ここで数えておけば、**足した本人がその場で気づきます。**
    // **`"name":` では数えられません。**引数の中にも `name` が在ります
    // （`register_connection` / `mark_connection`）。実際に 2 本多く数えました。
    // `inputSchema` は 1 本につきちょうど 1 つです。
    let counted = listed.matches("inputSchema").count();
    assert_eq!(
        counted, 35,
        "**MCP のツールが {counted} 本になりました。**足した／消したなら、\
         `.claude/project-status.md` の表と `CLAUDE.md` の本数も同じ commit で直してください\
         （product-baseline §10）。直したら、ここの数字も合わせてください"
    );

    // **ここが D3 の見張り。**引数で任意の文字列をシェルへ渡す口を 1 つも作らない。
    // **Phase 2 へ回した書き込みが、うっかり生えていないこと**（PRD §3）。
    // 上げるのは入れたが、消す・動かす・権限を変えるは入れていない。
    //
    // **`sudo` という名前の口も作りません**（D48 で `become = "ask"` を入れたあとも）。
    // 権限を上げられるのは `run_operation` の中だけで、**渡せるのは id**、
    // 走るのは**人が `operations.toml` に書いた文字列**、
    // **しかも人が画面で許可するまで走りません。**
    for forbidden in [
        "delete_file",
        "remove_file",
        "rename",
        "move_file",
        "chmod",
        "chown",
        "restart_service",
        "sudo",
    ] {
        let as_name = format!("\"name\":\"{forbidden}\"");
        assert!(
            !listed.contains(&as_name),
            "Phase 2 の口が生えている（{forbidden}）: {listed}"
        );
    }

    // **道具の名前で見る。**説明文に部分一致させると、無関係な文章に当たる
    // （`passphrase` で 1 度踏んだ。今度は "operating system" が `system` に当たった）。
    // 名前で見る方が**厳しく、かつ正確**。
    // **接続情報の受け渡しは AI に渡さない**（D18）。
    // バンドルは**接続先と鍵のパスフレーズを丸ごと**含みます。
    // 「AI の書き込みを囲いの外へ出さない」（D22）以前の話として、
    // **AI が触ってよいものではありません。**
    // 手元へ落とす口を渡さない（D27）のと同じ理由です。
    for forbidden in [
        "bundle_export",
        "bundle_import",
        "export_connections",
        "import_connections",
    ] {
        let as_name = format!("\"name\":\"{forbidden}\"");
        assert!(
            !listed.contains(&as_name),
            "接続情報を持ち出す口が生えている（{forbidden}）: {listed}"
        );
    }

    for forbidden in ["run_command", "shell", "system", "exec", "eval"] {
        let as_name = format!("\"name\":\"{forbidden}\"");
        assert!(
            !listed.contains(&as_name),
            "任意コマンドの口がある（{forbidden}）: {listed}"
        );
    }

    // **ここが D11 の見張り。**AI に秘密を渡す口を作らない。
    // **引数名だけを見る。**説明文には「パスフレーズを受け取らない」と書いてあるので、
    // 素朴に部分一致させると説明文に当たってしまう（実際に当たった）。
    for secret in [
        "passphrase",
        "password",
        "secret",
        "private_key",
        "credential",
    ] {
        let as_json_key = format!("\"{secret}\":");
        assert!(
            !listed.contains(&as_json_key),
            "秘密を受け取る引数が生えている（{secret}）: {listed}"
        );
    }

    endpoint.shutdown();
}

/// SSE で包まれて返るので、`data:` の行から JSON を取り出す。
fn payload(body: &str) -> serde_json::Value {
    for line in body.lines() {
        let raw = line.strip_prefix("data:").unwrap_or(line).trim();
        if raw.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
                return value;
            }
        }
    }
    panic!("JSON が見つからない: {body}");
}

/// 一覧から道具を 1 本、名前で引く。
fn tool<'a>(listed: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    listed["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools が配列でない: {listed}"))
        .iter()
        .find(|held| held["name"] == name)
        .unwrap_or_else(|| panic!("{name} が一覧に無い: {listed}"))
}

/// 道具の説明文。
fn described<'a>(listed: &'a serde_json::Value, name: &str) -> &'a str {
    tool(listed, name)["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{name} に説明が無い"))
}

#[tokio::test]
async fn no_tool_description_claims_only_one_connection_can_be_open() {
    // **説明文は、MCP を呼ぶ側にとって世界の全て**（Issue #9）。
    //
    // 実装は 2026-08-29 に複数対応へ変わったのに（D25 / `55288f2`）、
    // `connect` の説明だけが「1 本だけ」のまま **7 日間**残りました。
    // その間、説明を読んだエージェントは **2 本目を試さずに諦めます。**
    // 人は画面を触って気づけますが、**呼ぶ側は説明文しか持ちません。**
    //
    // 上の見張りは**名前だけ**を見ています（無関係な文章に当たるため、
    // それ自体は正しい判断です）。**説明文を見張らない理由にはなりません。**
    // Arrange
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    // Act
    let init = post(&client, &endpoint, None, INIT_BODY).await;
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_owned());
    let _ = init.text().await;
    post(&client, &endpoint, session.as_deref(), INITIALIZED_BODY).await;
    let body = post(&client, &endpoint, session.as_deref(), LIST_BODY)
        .await
        .text()
        .await
        .unwrap();
    let listed = payload(&body);

    // Assert
    let connect = described(&listed, "connect").to_lowercase();
    for lie in [
        "exactly one connection",
        "one connection at a time",
        "only one connection",
        "a single connection",
        "one connection only",
    ] {
        assert!(
            !connect.contains(lie),
            "`connect` の説明が「1 本だけ」と言っている（{lie}）: {connect}"
        );
    }

    // **言い落としも防ぐ。**否定だけだと、複数持てることが 1 行も書かれていない
    // 説明でも通ります。**書いていないものは、呼ぶ側には無いのと同じ**です。
    assert!(
        connect.contains("several") || connect.contains("multiple"),
        "`connect` の説明に、複数持てることが書かれていない: {connect}"
    );

    // **宛先を変える口へ繋がっていること。**繋げると分かっても、
    // 切り替え方が書いていなければ 2 本目は使われません。
    assert!(
        connect.contains("focus_connection"),
        "`connect` の説明に、宛先の変え方（focus_connection）が無い: {connect}"
    );

    // **2 つの説明が食い違わないこと。**食い違いこそが Issue #9 の中身です。
    let focus = described(&listed, "focus_connection").to_lowercase();
    assert!(
        focus.contains("several") || focus.contains("multiple"),
        "`focus_connection` の説明から、複数持てることが消えている: {focus}"
    );

    endpoint.shutdown();
}

#[tokio::test]
async fn the_mcp_port_is_bound_to_loopback_only() {
    // 外から叩ける口を開けていないこと（PRD §8 / 21）。
    // Arrange & Act
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");

    // Assert
    assert!(
        endpoint.addr().ip().is_loopback(),
        "loopback でない: {}",
        endpoint.addr()
    );

    endpoint.shutdown();
}

#[tokio::test]
async fn a_caller_without_the_token_gets_nowhere() {
    // **同じ端末の別プロセスから叩ける口に、書き込みが載っている**（D23）。
    // 合言葉を知らない相手には、initialize すら通さない。
    // Arrange
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();
    let url = endpoint.url();

    // Act
    let bare = post_with(&client, &url, None, INIT_BODY, None).await;
    let wrong = post_with(
        &client,
        &url,
        None,
        INIT_BODY,
        Some("0".repeat(endpoint.token().len())),
    )
    .await;
    let right = post(&client, &endpoint, None, INIT_BODY).await;

    // Assert
    assert_eq!(bare.status(), 401, "合言葉なしで通っている");
    assert_eq!(wrong.status(), 401, "違う合言葉で通っている");
    assert_eq!(right.status(), 200, "正しい合言葉で通らない");

    // **何が違うのかを漏らさない。**総当たりの手掛かりを渡さない。
    let body = wrong.text().await.expect("応答が読めない");
    assert!(
        !body.contains(endpoint.token()),
        "応答に合言葉そのものが出ている: {body}"
    );

    endpoint.shutdown();
}

#[tokio::test]
async fn the_token_is_different_for_every_endpoint() {
    // 起動ごとに変わらないなら、1 回覗かれた時点で以後ずっと通る。
    let mut made = Vec::new();
    for _ in 0..2 {
        let endpoint = serve(ServeParts {
            band: Band::new(),
            stream: Arc::new(OutputStream::new()),
            connections_watch: Arc::new(ConnectionsWatch::new()),
            engine: None,
            capture: None,
            view: None,
            token: None,
            port: 0,
            ack_timeout: Duration::from_secs(5),
        })
        .await
        .expect("MCP が立ち上がらない");
        made.push(endpoint.token().to_string());
        endpoint.shutdown();
    }

    assert_ne!(made[0], made[1], "合言葉が使い回されている");
}

/// **省略したら伏せる**（D26）。
///
/// ここが `false` に倒れた瞬間、引数を書き忘れた呼び出しが接続先を写します。
/// 見えるのは道具の側の既定だけなので、**引数の定義そのものを見張ります。**
#[tokio::test]
async fn the_capture_tool_redacts_unless_it_is_told_otherwise() {
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    let init = post(&client, &endpoint, None, INIT_BODY).await;
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_owned());
    let _ = init.text().await;
    post(&client, &endpoint, session.as_deref(), INITIALIZED_BODY).await;
    let listed = post(&client, &endpoint, session.as_deref(), LIST_BODY)
        .await
        .text()
        .await
        .unwrap();

    // **`redact` は省略できる引数**でなければならない（必須にすると呼ぶ側が面倒がる）。
    // そのうえで、省略時の扱いが「伏せる」であることを説明文で約束している。
    assert!(listed.contains("capture_window"), "{listed}");
    assert!(
        listed.contains("redact"),
        "伏せるかどうかを選べない: {listed}"
    );
    assert!(
        listed
            .to_lowercase()
            .contains("by default the capture is redacted"),
        "既定で伏せることが道具の説明に書かれていない: {listed}"
    );

    // **画面が無いときは、正直に断る。**黙って伏せずに撮る、が一番危ない。
    let called = post(
        &client,
        &endpoint,
        session.as_deref(),
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"capture_window","arguments":{}}}"#,
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(
        called.contains("画面がありません"),
        "画面が無いのに撮ったことになっている: {called}"
    );
}

#[tokio::test]
async fn naming_itself_reaches_the_caller_and_leaves_the_band_alone() {
    // **作ったのに繋いでいない、を作らない**（Issue #10 の教訓）。
    // 一覧に名前が在ることと、呼んで中身が返ることは別です。
    //
    // 同時に、**帯へ 1 行も出ないこと**を見ます（D49）。
    // サーバーへ触らず、人に見せる価値のある動きではないので、
    // **名乗るたびに帯が増えると、本当の操作が埋もれます。**
    let band = Band::new();
    let mut watching = band.subscribe();
    let endpoint = serve(ServeParts {
        band: band.clone(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    let init = post(&client, &endpoint, None, INIT_BODY).await;
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_owned());
    let _ = init.text().await;

    let answer = post(
        &client,
        &endpoint,
        session.as_deref(),
        r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"about_sshboard","arguments":{}}}"#,
    )
    .await
    .text()
    .await
    .expect("応答が読めない");

    // **走っている版を名乗る。**
    let running = env!("CARGO_PKG_VERSION");
    assert!(answer.contains(running), "版を名乗っていない: {answer}");
    // **できないことを言う。**ここが空だと、AI は断られてから学び直します。
    assert!(
        answer.contains("no tool that takes a shell command"),
        "できないことを言っていない: {answer}"
    );
    // **版ごとの変更が入っている。**
    assert!(
        answer.contains("history"),
        "変更履歴が入っていない: {answer}"
    );
    assert!(answer.contains("0.1.0"), "最初の版まで辿れない: {answer}");

    // **帯には 1 行も出さない。**
    // 待ってから見ます —— **即座に見ると「まだ来ていないだけ」と区別が付きません。**
    let quiet = tokio::time::timeout(Duration::from_millis(300), watching.recv()).await;
    assert!(
        quiet.is_err(),
        "名乗るだけで帯へ出ている（本当の操作が埋もれます）: {quiet:?}"
    );

    endpoint.shutdown();
}

#[tokio::test]
async fn a_version_nobody_knows_is_refused_rather_than_answered_with_nothing() {
    // **黙って空を返さない**（product-baseline §8）。
    // 空だと「変わっていません」と読まれ、**打ち間違いに誰も気づけません。**
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    let init = post(&client, &endpoint, None, INIT_BODY).await;
    let session = init
        .headers()
        .get("mcp-session-id")
        .map(|v| v.to_str().unwrap().to_owned());
    let _ = init.text().await;

    let answer = post(
        &client,
        &endpoint,
        session.as_deref(),
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"about_sshboard","arguments":{"since":"9.9.9"}}}"#,
    )
    .await
    .text()
    .await
    .expect("応答が読めない");

    assert!(
        answer.contains("no version") && answer.contains("9.9.9"),
        "知らない版を黙って通している: {answer}"
    );

    endpoint.shutdown();
}

#[tokio::test]
async fn the_server_says_what_it_is_before_any_tool_is_called() {
    // **`instructions` は、呼ばなくてもクライアントが AI へ渡します。**
    // `about_sshboard` を作っても、**呼ばれなければ届きません。**
    //
    // ここには「入口」だけを置きます —— 全部書くと、
    // **繋いだ全員の文脈を毎回それだけ食います。**
    let endpoint = serve(ServeParts {
        band: Band::new(),
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();

    let said = post(&client, &endpoint, None, INIT_BODY)
        .await
        .text()
        .await
        .expect("応答が読めない");

    // **人が見ていることを最初に言う。**ここが伝わらないと、
    // AI は「誰も見ていない所で動いている」前提で振る舞います。
    assert!(
        said.contains("watches") || said.contains("watching"),
        "人が見ていることを言っていない: {said}"
    );
    // **既定が空なのは故障ではない**、を最初に言う。
    // 言わないと、断られた AI は「壊れている」と報告します。
    assert!(said.contains("EMPTY"), "既定が空だと言っていない: {said}");
    // **自分の名前を名乗る。**既定のままだと `rmcp`（ライブラリ名）を名乗り、
    // 繋いだ AI からは**何のサーバーなのか分かりません。**
    assert!(
        said.contains("\"name\":\"sshboard\""),
        "sshboard と名乗っていない: {said}"
    );
    assert!(
        said.contains(env!("CARGO_PKG_VERSION")),
        "版を名乗っていない: {said}"
    );
    // **続きの読み方を示す。**
    assert!(
        said.contains("about_sshboard"),
        "全体像の読み方を示していない: {said}"
    );

    endpoint.shutdown();
}
