//! **エージェント専用カーソル（`screen`）の口**（2026-11-12・オーナーの依頼・tadori の仕様が正本）。
//!
//! 画面の中だけで「どこを指して何を押したか」を見せる。人の OS のマウスとキーは使わない。
//!
//! sshboard では tadori より狭くします（GUI は制限の無い人の口なので・D22）。
//!
//! - **押せるのは、画面が「エージェントが押してよい」と印を付けた物だけ**（いまはタブ）
//! - **打つ（`type`）は入れない**。承認・パスフレーズ・端末の錠（D29）を迂回させない
//! - 画面に写る接続先は**探す対象から外す**（見つかる・見つからないで当てられるため・禁止事項 4）
//!
//! 画面（Tauri）はここに無いので、**偽の画面**で口の形だけを見ます。

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_mcp::{serve, ActOutcome, AgentAct, McpEndpoint, ServeParts, ShowView};
use sshboard_stream::OutputStream;

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

/// 偽の画面。**届いた指示を控え、決めておいた答えを返します。**
struct FakeScreen {
    answer: ActOutcome,
    got: Mutex<Vec<AgentAct>>,
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

fn fake_screen_band(band: &Band) {
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
}

impl Harness {
    async fn post(&self, body: String) -> String {
        let mut request = self
            .client
            .post(self.endpoint.url())
            .header("Authorization", format!("Bearer {}", self.endpoint.token()))
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

    async fn screen(&self, arguments: serde_json::Value) -> String {
        self.post(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": { "name": "screen", "arguments": arguments }
            })
            .to_string(),
        )
        .await
    }
}

async fn harness(view: Option<Arc<dyn ShowView>>) -> Harness {
    let band = Band::new();
    fake_screen_band(&band);
    let endpoint = serve(ServeParts {
        band,
        stream: Arc::new(OutputStream::new()),
        connections_watch: Arc::new(ConnectionsWatch::new()),
        engine: None,
        capture: None,
        view,
        token: None,
        port: 0,
        ack_timeout: Duration::from_secs(5),
    })
    .await
    .expect("MCP が立ち上がらない");
    let client = reqwest::Client::new();
    let mut harness = Harness {
        endpoint,
        client,
        session: None,
    };
    let init = harness
        .client
        .post(harness.endpoint.url())
        .header(
            "Authorization",
            format!("Bearer {}", harness.endpoint.token()),
        )
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(INIT)
        .send()
        .await
        .expect("MCP へ届かない");
    harness.session = init
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let _ = init.text().await;
    harness.post(INITIALIZED.to_string()).await;
    harness
}

fn fake(answer: ActOutcome) -> Arc<FakeScreen> {
    Arc::new(FakeScreen {
        answer,
        got: Mutex::new(Vec::new()),
    })
}

fn refused(reply: &str) -> bool {
    reply.contains("\"isError\":true") || reply.contains("\"error\"")
}

#[tokio::test]
async fn without_a_screen_it_says_to_open_one_first() {
    // Arrange
    let harness = harness(None).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "point", "target": "Files" }))
        .await;

    // Assert
    assert!(refused(&reply), "画面が無いのに通った: {reply}");
    assert!(
        reply.contains("open"),
        "画面を開いてから、と言っていない: {reply}"
    );
}

#[tokio::test]
async fn typing_is_refused_before_it_reaches_the_screen() {
    // Arrange —— 打つ口は入れない（承認・パスフレーズ・端末の錠を迂回させない）。
    let screen = fake(ActOutcome::Done);
    let harness = harness(Some(screen.clone())).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "type", "target": "Host", "value": "x" }))
        .await;

    // Assert
    assert!(refused(&reply), "打つ指示が通った: {reply}");
    assert!(
        screen.got.lock().unwrap().is_empty(),
        "断ったはずの指示が画面へ届いた"
    );
}

#[tokio::test]
async fn pointing_at_something_on_screen_reaches_the_screen() {
    // Arrange
    let screen = fake(ActOutcome::Done);
    let harness = harness(Some(screen.clone())).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "point", "target": "Files", "message": "ここです" }))
        .await;

    // Assert
    assert!(!refused(&reply), "指せたのに断っている: {reply}");
    let got = screen.got.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![AgentAct::Point {
            target: "Files".into(),
            message: Some("ここです".into()),
            seconds: None,
        }]
    );
}

#[tokio::test]
async fn something_not_on_screen_is_reported_and_nothing_is_pressed() {
    // Arrange
    let harness = harness(Some(fake(ActOutcome::NotFound))).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "click", "target": "No such button" }))
        .await;

    // Assert
    assert!(refused(&reply), "見つからないのに成功と言った: {reply}");
    assert!(
        reply.contains("not found"),
        "見つからない、と言っていない: {reply}"
    );
}

#[tokio::test]
async fn pressing_something_not_marked_for_agents_is_refused() {
    // Arrange —— 画面が「押してよい」と印を付けていない物（承認の釦など）。
    let harness = harness(Some(fake(ActOutcome::NotAllowed))).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "click", "target": "Allow" }))
        .await;

    // Assert
    assert!(
        refused(&reply),
        "押してよくない物を押したことにした: {reply}"
    );
    assert!(
        reply.contains("person"),
        "人が押すものだ、と言っていない: {reply}"
    );
}

#[tokio::test]
async fn going_to_a_view_that_does_not_exist_is_refused() {
    // Arrange
    let screen = fake(ActOutcome::Done);
    let harness = harness(Some(screen.clone())).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "go", "view": "settings" }))
        .await;

    // Assert
    assert!(refused(&reply), "無い画面へ行かせた: {reply}");
    assert!(screen.got.lock().unwrap().is_empty());
}

#[tokio::test]
async fn seconds_outside_one_to_sixty_are_refused() {
    // Arrange
    let screen = fake(ActOutcome::Done);
    let harness = harness(Some(screen.clone())).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "say", "message": "hi", "seconds": 600 }))
        .await;

    // Assert
    assert!(refused(&reply), "600 秒を通した: {reply}");
    assert!(screen.got.lock().unwrap().is_empty());
}

#[tokio::test]
async fn while_the_person_is_busy_on_screen_nothing_is_taken_from_them() {
    // Arrange —— 人が問いに答えている・欄に打っている最中（`canFollow` が偽）。
    let harness = harness(Some(fake(ActOutcome::Busy))).await;

    // Act
    let reply = harness
        .screen(serde_json::json!({ "action": "go", "view": "files" }))
        .await;

    // Assert —— 画面を奪わない。**何もしていないこと**と、待てばよいことを言う。
    assert!(
        refused(&reply),
        "人の作業中に画面を動かしたことにした: {reply}"
    );
    assert!(
        reply.contains("busy"),
        "人が使っている、と言っていない: {reply}"
    );
}
