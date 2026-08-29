//! Issue 001 の完了条件を、**外部クライアントと同じ経路**で確かめる。
//!
//! rmcp のクライアント SDK を使わず、生の JSON-RPC を HTTP へ投げる。
//! SDK を挟むと「SDK 同士が話せた」ことしか分からず、
//! 別実装の MCP クライアントで動く保証にならない。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_connections::ConnectionsWatch;
use sshboard_mcp::{serve, McpEndpoint};
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
    let endpoint = serve(
        band,
        Arc::new(OutputStream::new()),
        Arc::new(ConnectionsWatch::new()),
        None,
        None,
        0,
        Duration::from_secs(5),
    )
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
    let endpoint = serve(
        Band::new(),
        Arc::new(OutputStream::new()),
        Arc::new(ConnectionsWatch::new()),
        None,
        None,
        0,
        Duration::from_secs(5),
    )
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
    ] {
        assert!(
            listed.contains(expected),
            "{expected} が一覧に無い: {listed}"
        );
    }

    // **ここが D3 の見張り。**引数で任意の文字列をシェルへ渡す口を 1 つも作らない。
    // **Phase 2 へ回した書き込みが、うっかり生えていないこと**（PRD §3）。
    // 上げるのは入れたが、消す・動かす・権限を変えるは入れていない。
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
        assert!(
            !listed.contains(forbidden),
            "Phase 2 の口が生えている（{forbidden}）: {listed}"
        );
    }

    for forbidden in ["run_command", "shell", "system"] {
        assert!(
            !listed.contains(forbidden),
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

#[tokio::test]
async fn the_mcp_port_is_bound_to_loopback_only() {
    // 外から叩ける口を開けていないこと（PRD §8 / 21）。
    // Arrange & Act
    let endpoint = serve(
        Band::new(),
        Arc::new(OutputStream::new()),
        Arc::new(ConnectionsWatch::new()),
        None,
        None,
        0,
        Duration::from_secs(5),
    )
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
    let endpoint = serve(
        Band::new(),
        Arc::new(OutputStream::new()),
        Arc::new(ConnectionsWatch::new()),
        None,
        None,
        0,
        Duration::from_secs(5),
    )
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
        let endpoint = serve(
            Band::new(),
            Arc::new(OutputStream::new()),
            Arc::new(ConnectionsWatch::new()),
            None,
            None,
            0,
            Duration::from_secs(5),
        )
        .await
        .expect("MCP が立ち上がらない");
        made.push(endpoint.token().to_string());
        endpoint.shutdown();
    }

    assert_ne!(made[0], made[1], "合言葉が使い回されている");
}
