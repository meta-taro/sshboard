//! Issue 001 の完了条件を、ヘッドレスで押さえるテスト。
//!
//! HTTP を経由しない。ここで見るのは「ツール応答より先に帯へ出るか」だけ。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_mcp::SshboardMcp;
use sshboard_stream::OutputStream;

#[tokio::test]
async fn ping_answers_pong() {
    // Arrange
    let server = SshboardMcp::new(Band::new(), Arc::new(OutputStream::new()));

    // Act
    let answer = server.ping().await.expect("ping が失敗した");

    // Assert
    assert_eq!(answer, "pong");
}

#[tokio::test]
async fn ping_puts_an_ai_line_on_the_band() {
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    tokio::spawn(async move { server.ping().await });
    let event = subscriber.recv().await.expect("帯へ出ていない");

    // Assert
    assert_eq!(event.line().actor(), Actor::Ai);
    assert_eq!(event.line().text(), "ping");
    assert!(
        event.line().render().starts_with("[AI]"),
        "実際: {:?}",
        event.line().render()
    );
}

#[tokio::test]
async fn ping_does_not_answer_until_the_band_has_the_line() {
    // 001 の本命。AI が返答したあとに画面が追いつく形になっていないこと。
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    let mut call = tokio::spawn(async move { server.ping().await });
    let answered_before_the_band = tokio::time::timeout(Duration::from_millis(100), &mut call)
        .await
        .is_ok();

    subscriber.recv().await.expect("帯へ出ていない").ack();
    let answer = tokio::time::timeout(Duration::from_secs(2), call)
        .await
        .expect("ack したのに応答が返らない")
        .expect("ツールがパニックした")
        .expect("ping が失敗した");

    // Assert
    assert!(
        !answered_before_the_band,
        "帯が受け取る前に応答が返っている"
    );
    assert_eq!(answer, "pong");
}

#[tokio::test]
async fn ping_fails_when_the_screen_never_confirms() {
    // 画面が固まっているのに応答だけ返すと、見えないまま操作が進む。
    // Arrange
    let band = Band::new();
    let _stuck_screen = band.subscribe(); // 受け取るが ack しない
    let server = SshboardMcp::with_ack_timeout(
        band,
        Arc::new(OutputStream::new()),
        Duration::from_millis(50),
    );

    // Act
    let result = server.ping().await;

    // Assert
    assert!(
        result.is_err(),
        "帯が受け取っていないのに成功している: {result:?}"
    );
}

#[tokio::test]
async fn read_stream_returns_the_plain_tail_without_any_ansi() {
    // GUI には色が残り、MCP には残らない（Issue 005）。
    // Arrange
    let stream = Arc::new(OutputStream::new());
    stream
        .push(b"\x1b[31mdisk full\x1b[0m\r\n")
        .expect("流せない");
    let server = SshboardMcp::new(Band::new(), Arc::clone(&stream));

    // Act
    let text = server.read_stream().await.expect("読めない");

    // Assert
    assert_eq!(text, "disk full\n");
    assert!(!text.contains('\x1b'), "ANSI が混ざっている: {text:?}");
}

#[tokio::test]
async fn read_stream_puts_an_ai_line_on_the_band_before_answering() {
    // 読んだことも帯に出る（PRD §4-2）。
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    let mut call = tokio::spawn(async move { server.read_stream().await });
    let answered_before_the_band = tokio::time::timeout(Duration::from_millis(100), &mut call)
        .await
        .is_ok();
    let event = subscriber.recv().await.expect("帯へ出ていない");
    let line = event.line().clone();
    event.ack();
    call.await.expect("パニック").expect("読めない");

    // Assert
    assert!(
        !answered_before_the_band,
        "帯が受け取る前に応答が返っている"
    );
    assert_eq!(line.actor(), Actor::Ai);
    assert_eq!(line.text(), "read_stream");
}
