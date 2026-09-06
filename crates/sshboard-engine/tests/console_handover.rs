//! 端末の受け渡し（D29）。**サーバーを一切使いません。**
//!
//! D29 はこう書いています。
//!
//! > **AI が握ったまま離さない状態を作らない。人の解除が常に勝つ。**
//!
//! **守っていたのは片側だけ**でした。人が締め出される側は守られていますが、
//! **AI が人から奪う側が守られていません。**
//!
//! `console_open` は「他が握っていたら断る」と正しく書いてあるのに、
//! `console_stop` が `actor` を受け取らず、**誰の握りでも外せます。**
//! そして MCP はそれを AI に開いています。
//!
//! ```text
//! AI: console_stop   → 人のシェルが落ち、握りが外れる
//! AI: console_open   → AI が握る
//! ```
//!
//! **2 手で取れます。**実機の指摘（2026-09-06）はこうでした。
//!
//! > AI に端末を渡すときに「止める」を押さないといけません。
//! > これだと **AI からのアクションが分からない**ので、
//! > 「AI が操作をするために許可しますか」みたいなアラートで人に気づかせないと。
//!
//! **分からないどころか、止められません。**ここを先に塞ぎます。

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_stream::OutputStream;

/// 接続一覧だけ置いた Engine。**1 件も繋いでいません。**
fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn the_ai_cannot_stop_a_console_the_human_is_holding() {
    // **これが裏口でした。**`console_open` で奪えないことは確かめてあるのに、
    // `console_stop` で外してから開けば、同じことができました。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    engine
        .console_take(Actor::Human)
        .await
        .expect("人が握れない");

    // Act
    let refused = engine.console_stop(Actor::Ai).await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::ConsoleHeldByOther { .. })),
        "AI が人の端末を止められてしまう: {refused:?}"
    );
    assert_eq!(
        engine.console_holder().await,
        Some(Actor::Human),
        "握りが人から外れている"
    );
}

#[tokio::test]
async fn the_human_can_always_stop_it_even_when_the_ai_holds_it() {
    // **人の解除が常に勝つ**（D29）。ここは 1 ミリも緩めません。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    engine.console_take(Actor::Ai).await.expect("AI が握れない");

    // Act
    let stopped = engine.console_stop(Actor::Human).await;

    // Assert
    assert!(stopped.is_ok(), "人が止められない: {stopped:?}");
    assert_eq!(
        engine.console_holder().await,
        None,
        "止めたのに握りが残っている"
    );
}

#[tokio::test]
async fn the_ai_can_let_go_of_what_it_holds_itself() {
    // **自分の分は畳める。**畳めないと AI は握りっぱなしになります
    // （D29 が一番避けたい形）。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    engine.console_take(Actor::Ai).await.expect("AI が握れない");

    // Act
    let stopped = engine.console_stop(Actor::Ai).await;

    // Assert
    assert!(stopped.is_ok(), "AI が自分の分を畳めない: {stopped:?}");
    assert_eq!(engine.console_holder().await, None);
}

#[tokio::test]
async fn stopping_a_console_nobody_holds_is_not_an_error() {
    // **同じ状態へ向かう操作**なので失敗にしません（`disconnect` と同じ立場）。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act & Assert
    assert!(engine.console_stop(Actor::Ai).await.is_ok());
    assert!(engine.console_stop(Actor::Human).await.is_ok());
}
