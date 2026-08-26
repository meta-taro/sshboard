//! 1 本の出力が両方の面へ同時に出ることのテスト（Issue 005）。

use sshboard_stream::{OutputStream, StreamStopped};

#[tokio::test]
async fn one_push_reaches_both_faces() {
    // Arrange
    let stream = OutputStream::new();
    let mut raw = stream.subscribe_raw();
    let mut plain = stream.subscribe_plain();

    // Act — 1 回だけ流す
    stream
        .push(b"\x1b[31mERROR\x1b[0m disk full\n")
        .expect("流せない");

    // Assert
    let raw_chunk = raw.recv().await.expect("GUI 側へ来ていない");
    let plain_chunk = plain.recv().await.expect("MCP 側へ来ていない");

    assert_eq!(
        raw_chunk, b"\x1b[31mERROR\x1b[0m disk full\n",
        "GUI 側の色が落ちている"
    );
    assert_eq!(plain_chunk, "ERROR disk full\n");
}

#[tokio::test]
async fn the_mcp_side_never_carries_an_escape_byte() {
    // Arrange
    let stream = OutputStream::new();
    let mut plain = stream.subscribe_plain();

    // Act
    stream
        .push(b"\x1b]0;title\x07\x1b[1;32mok\x1b[0m\r\n")
        .expect("流せない");

    // Assert
    let text = plain.recv().await.expect("来ていない");
    assert!(!text.contains('\x1b'), "ANSI が混ざっている: {text:?}");
    assert_eq!(text, "ok\n");
}

#[tokio::test]
async fn a_stopped_stream_refuses_to_carry_anything_more() {
    // 人が AI の実行を止められること（PRD §4-3）。
    // Arrange
    let stream = OutputStream::new();
    let mut plain = stream.subscribe_plain();
    stream.push(b"before\n").expect("流せない");

    // Act
    stream.stop();
    let after = stream.push(b"after\n");

    // Assert
    assert_eq!(after, Err(StreamStopped));
    assert!(stream.is_stopped());
    assert_eq!(plain.recv().await.expect("来ていない"), "before\n");
    assert!(plain.try_recv().is_err(), "止めたあとの分が流れている");
}

#[tokio::test]
async fn a_face_that_nobody_is_watching_does_not_stop_the_other() {
    // 画面を閉じても MCP 側は流れ続ける（逆も同じ）。
    // Arrange
    let stream = OutputStream::new();
    let mut plain = stream.subscribe_plain();

    // Act — GUI 側は誰も購読していない
    stream.push(b"only mcp\n").expect("流せない");

    // Assert
    assert_eq!(plain.recv().await.expect("来ていない"), "only mcp\n");
}
