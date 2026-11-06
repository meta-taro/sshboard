//! **人が答えたことを、AI のセッションへ押し出す**（Issue #29・Claude Code の channels）。
//!
//! `await_answer` は AI のターンが動いている間しか待てません。**ターンを閉じたあとに
//! 人が押すと、AI は気づけませんでした。**Claude Code は `claude/channel` を名乗った口からの
//! `notifications/claude/channel` を、入力待ちのセッションにも届けます。
//!
//! 1. **名乗る**（名乗らないと、Claude Code は受けない）
//! 2. **解けたときだけ知らせる**（新しく問いが立っただけでは起こさない —— AI は自分で問うた）
//! 3. **何が解けたかを言う。接続先は言わない**（禁止事項 4）

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_mcp::{
    answered, channel_notice, serve, ServeParts, Waiting, CHANNEL_CAPABILITY, CHANNEL_METHOD,
};
use sshboard_stream::OutputStream;

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;

#[tokio::test]
async fn the_server_says_it_can_push_to_a_claude_code_channel() {
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

    let said = reqwest::Client::new()
        .post(endpoint.url())
        .header("Authorization", format!("Bearer {}", endpoint.token()))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(INIT_BODY)
        .send()
        .await
        .expect("送れない")
        .text()
        .await
        .expect("読めない");

    assert_eq!(CHANNEL_CAPABILITY, "claude/channel");
    assert!(
        said.contains("\"claude/channel\""),
        "**名乗っていない**（名乗らないと Claude Code は押し出しを受けません）: {said}"
    );
    endpoint.shutdown();
}

#[test]
fn a_question_that_was_just_asked_wakes_nobody() {
    let before = Waiting::default();
    let after = Waiting {
        operation: true,
        ..Waiting::default()
    };
    assert!(answered(before, after).is_empty());
}

#[test]
fn a_question_that_went_away_is_reported_by_kind() {
    let before = Waiting {
        operation: true,
        passphrase: true,
        ..Waiting::default()
    };
    let after = Waiting {
        passphrase: true,
        ..Waiting::default()
    };
    assert_eq!(answered(before, after), vec!["operation"]);
}

#[test]
fn the_notice_names_what_was_answered_and_what_to_do_next() {
    assert_eq!(CHANNEL_METHOD, "notifications/claude/channel");
    let notice = channel_notice(&["operation", "console"]).expect("知らせが無い");

    let content = notice["content"].as_str().expect("content は文字");
    assert!(content.contains("operation"), "{content}");
    assert!(
        content.contains("pending_status"),
        "次に何をするかを言っていない: {content}"
    );
    assert_eq!(notice["meta"]["answered"], "operation,console");
    assert!(channel_notice(&[]).is_none(), "解けていないのに知らせた");
}

/// 帯へ来た行を全部 ack する（画面の代わり）。
fn ack_everything(band: &Band) {
    let mut subscriber = band.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            event.ack();
        }
    });
}

#[tokio::test]
async fn when_the_person_answers_on_screen_the_notice_arrives_on_the_listening_stream() {
    // **通しで。**テスト用サーバー（`sh tools/test-server/up.sh`）が無ければ飛ばします。
    if tokio::net::TcpStream::connect(("127.0.0.1", 2222))
        .await
        .is_err()
    {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let connections = dir.path().join("connections.toml");
    // **指紋を書かない**＝初めての相手。繋ぐとホスト鍵の問いが立つ。
    std::fs::write(
        &connections,
        "version = 1\n\n[[connections]]\nid = \"local\"\nname = \"Local\"\n\
         host = \"127.0.0.1\"\nport = 2222\nuser = \"probe\"\n",
    )
    .expect("接続一覧を書けない");
    let band = Band::new();
    ack_everything(&band);
    let stream = Arc::new(OutputStream::new());
    let engine = Arc::new(sshboard_engine::Engine::new(
        band.clone(),
        Arc::clone(&stream),
        connections,
    ));
    let endpoint = serve(ServeParts {
        band,
        stream,
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: Some(Arc::clone(&engine)),
        capture: None,
        view: None,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();
    let bearer = format!("Bearer {}", endpoint.token());

    let init = client
        .post(endpoint.url())
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(INIT_BODY)
        .send()
        .await
        .expect("送れない");
    let session = init.headers()["mcp-session-id"]
        .to_str()
        .expect("番号")
        .to_owned();
    let _ = init.text().await;
    let _ = client
        .post(endpoint.url())
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session)
        .body(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
        .send()
        .await
        .expect("送れない");

    // **呼んでいないのに来るもの**を受ける流れ（中継が開けるのと同じもの）。
    let mut listening = client
        .get(endpoint.url())
        .header("Authorization", &bearer)
        .header("Accept", "text/event-stream")
        .header("mcp-session-id", &session)
        .send()
        .await
        .expect("流れを開けない");
    assert!(
        listening.status().is_success(),
        "流れを開けない: {}",
        listening.status()
    );

    let refused = engine
        .connect(sshboard_band::Actor::Ai, "local", None)
        .await;
    assert!(
        matches!(
            refused,
            Err(sshboard_engine::EngineError::UntrustedHost { .. })
        ),
        "ホスト鍵の問いが立っていない: {refused:?}"
    );
    // **人が答えるまでには間がある。**問いが立ったことを見張りが見てから答える
    // （立って 0 秒で消えた問いは、見張りから見ると「立たなかった」のと同じ）。
    tokio::time::sleep(Duration::from_millis(300)).await;
    // 人が画面で断る ＝ 問いが解ける。
    engine
        .refuse_host_key(sshboard_band::Actor::Human, "local")
        .await
        .expect("断れない");

    let mut got = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !got.contains(CHANNEL_METHOD) && tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), listening.chunk()).await {
            Ok(Ok(Some(chunk))) => got.push_str(&String::from_utf8_lossy(&chunk)),
            Ok(_) => break,
            Err(_) => {}
        }
    }

    assert!(got.contains(CHANNEL_METHOD), "知らせが流れに来ない: {got}");
    assert!(
        got.contains("host_key"),
        "何が解けたかを言っていない: {got}"
    );
    assert!(
        !got.contains("127.0.0.1"),
        "**接続先が載っている**（禁止事項 4）: {got}"
    );
    endpoint.shutdown();
}
