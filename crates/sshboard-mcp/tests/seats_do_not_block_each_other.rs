//! **ある席の遅い呼びが、ほかの席を止めない**（2026-11-11）。
//!
//! 1 台の PC で何人ものエージェント（席）が同じ sshboard を叩きます。
//! 2026-11-11、ある席の長い `search` が返らない間に、**別の席の
//! `list_connections`（手元の一覧を読むだけ）まで 2 分以上返らなく**なりました
//! （動いていたのは 0.1.27）。
//!
//! 席ごとに中継は別で、中継の中で並べて流すことは `stdio_proxy` の試験が見ています。
//! **本体の側で、会期どうしが互いを待たないこと**はどこも見ていなかったので、ここで見ます。
//!
//! 遅い呼びは、**画面が応えない呼び**で作ります（SSH の要らない、確実に待つ呼び）。

use std::sync::Arc;
use std::time::{Duration, Instant};

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_mcp::{serve, McpEndpoint, ServeParts};
use sshboard_stream::OutputStream;

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"sshboard-test","version":"0"}}}"#;
const INITIALIZED_BODY: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const SLOW_BODY: &str =
    r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ping","arguments":{}}}"#;
const QUICK_BODY: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_connections","arguments":{}}}"#;

/// 遅い席が待たされる長さ。**画面が応えないとき、ここまで待ってから断ります。**
const SLOW_CALL_WAITS: Duration = Duration::from_secs(4);

/// 別の席の呼びが返るまでに許す長さ。遅い席の待ちより十分短くします。
const QUICK_CALL_LIMIT: Duration = Duration::from_secs(1);

/// 遅い呼びが本体に届いて、待ち始めるまでの余裕。
const SLOW_CALL_HEAD_START: Duration = Duration::from_millis(300);

/// 画面の代わり。**`ping` の行にだけ応えません**（遅い席を作るため）。ほかは全部 ack します。
fn screen_that_ignores_ping(band: &Band) -> tokio::task::JoinHandle<()> {
    let mut subscriber = band.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            if !event.line().text().contains("ping") {
                event.ack();
            }
        }
    })
}

async fn post(
    client: &reqwest::Client,
    endpoint: &McpEndpoint,
    session: Option<&str>,
    body: &'static str,
) -> reqwest::Response {
    let mut request = client
        .post(endpoint.url())
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", format!("Bearer {}", endpoint.token()))
        .body(body);
    if let Some(id) = session {
        request = request.header("Mcp-Session-Id", id);
    }
    request.send().await.expect("MCP へ届かない")
}

/// 1 席ぶんの会期を張る。**席ごとに別の会期**です（中継が席ごとに別なのと同じ）。
async fn open_seat(client: &reqwest::Client, endpoint: &McpEndpoint) -> String {
    let init = post(client, endpoint, None, INIT_BODY).await;
    assert_eq!(init.status(), 200, "initialize が通らない");
    let session = init
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .expect("会期の番号が返らない");
    let _ = init.text().await;
    let ack = post(client, endpoint, Some(&session), INITIALIZED_BODY).await;
    assert!(ack.status().is_success(), "initialized が通らない");
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_slow_call_on_one_seat_does_not_hold_up_another_seat() {
    // Arrange
    let band = Band::new();
    let _screen = screen_that_ignores_ping(&band);
    let endpoint = Arc::new(
        serve(ServeParts {
            band,
            stream: Arc::new(OutputStream::new()),
            connections_watch: Arc::new(ConnectionsWatch::new()),
            engine: None,
            capture: None,
            view: None,
            token: None,
            port: 0,
            ack_timeout: SLOW_CALL_WAITS,
        })
        .await
        .expect("MCP が立ち上がらない"),
    );
    let client = reqwest::Client::new();
    let slow_seat = open_seat(&client, &endpoint).await;
    let quick_seat = open_seat(&client, &endpoint).await;

    let slow = {
        let client = client.clone();
        let endpoint = Arc::clone(&endpoint);
        tokio::spawn(async move {
            let started = Instant::now();
            let _ = post(&client, &endpoint, Some(&slow_seat), SLOW_BODY)
                .await
                .text()
                .await;
            started.elapsed()
        })
    };
    tokio::time::sleep(SLOW_CALL_HEAD_START).await;

    // Act
    let started = Instant::now();
    let quick = tokio::time::timeout(
        QUICK_CALL_LIMIT,
        post(&client, &endpoint, Some(&quick_seat), QUICK_BODY),
    )
    .await;
    let quick_took = started.elapsed();

    // Assert
    let quick = quick.unwrap_or_else(|_| {
        panic!("別の席の list_connections が {QUICK_CALL_LIMIT:?} 以内に返らない（遅い席に止められた）")
    });
    let body = quick.text().await.expect("応答が読めない");
    assert!(
        !body.contains("\"error\""),
        "別の席の list_connections が失敗した: {body}"
    );
    let slow_took = slow.await.expect("遅い席の呼びがパニックした");
    assert!(
        slow_took >= SLOW_CALL_WAITS - SLOW_CALL_HEAD_START,
        "遅い席が本当に待たされていない（{slow_took:?}）—— この試験は何も見ていないことになる"
    );
    assert!(
        quick_took < QUICK_CALL_LIMIT,
        "別の席が {quick_took:?} 待たされた"
    );

    endpoint.shutdown();
}
