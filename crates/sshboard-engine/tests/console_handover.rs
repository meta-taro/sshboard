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

/// AI に握らせる。**D42 が入ったので、頼んで人が許すまで握れません。**
async fn ai_holding(engine: &Engine) {
    let asked = engine.console_take(Actor::Ai).await;
    assert!(
        matches!(asked, Err(EngineError::ConsoleApprovalNeeded)),
        "AI が許可なく握れてしまう: {asked:?}"
    );
    engine
        .console_answer(Actor::Human, true)
        .await
        .expect("人が許せない");
    assert_eq!(engine.console_holder().await, Some(Actor::Ai));
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
    ai_holding(&engine).await;

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
    ai_holding(&engine).await;

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

// --- D42: AI が握るには人の許可が要る ------------------------------------
//
// 実機の指摘（2026-09-06）:
//
// > AI に端末を渡すときに「止める」を押さないといけません。
// > これだと **AI からのアクションが分からない**ので、
// > 「AI が操作をするために許可しますか」みたいなアラートで人に気づかせないと。
//
// **押させる形をやめます。**AI が握りたいときは、AI から頼み、**人が答えます。**
// 人の側は 1 ミリも変わりません（許可も要らず、いつでも取り返せる）。

#[tokio::test]
async fn the_ai_cannot_take_the_console_without_being_allowed() {
    // **誰も握っていなくても、勝手には握らせません。**
    // 「握り手が居ないなら黙って取れる」だと、**人は AI が触ったことに気づけません。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    let refused = engine.console_take(Actor::Ai).await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::ConsoleApprovalNeeded)),
        "AI が黙って握れてしまう: {refused:?}"
    );
    assert_eq!(engine.console_holder().await, None);
}

#[tokio::test]
async fn asking_leaves_a_request_the_screen_can_show() {
    // **画面に出せない要求は、無いのと同じ**です。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    let _ = engine.console_take(Actor::Ai).await;

    // Assert
    assert_eq!(
        engine.console_request().await,
        Some(Actor::Ai),
        "頼んだことが残っていない"
    );
}

#[tokio::test]
async fn asking_twice_does_not_stack_up_into_nagging() {
    // **催促を積み上げさせない。**AI が何度呼んでも、人に出る問いは 1 つ。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    for _ in 0..5 {
        let _ = engine.console_take(Actor::Ai).await;
    }

    // Assert
    assert_eq!(engine.console_request().await, Some(Actor::Ai));
}

#[tokio::test]
async fn the_human_allowing_it_hands_the_console_over() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    let _ = engine.console_take(Actor::Ai).await;

    // Act
    engine
        .console_answer(Actor::Human, true)
        .await
        .expect("人が許せない");

    // Assert
    assert_eq!(engine.console_holder().await, Some(Actor::Ai));
    assert_eq!(engine.console_request().await, None, "問いが残っている");
}

#[tokio::test]
async fn the_human_refusing_it_leaves_the_console_where_it_was() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    engine
        .console_take(Actor::Human)
        .await
        .expect("人が握れない");
    let _ = engine.console_take(Actor::Ai).await;

    // Act
    engine
        .console_answer(Actor::Human, false)
        .await
        .expect("人が断れない");

    // Assert
    assert_eq!(
        engine.console_holder().await,
        Some(Actor::Human),
        "断ったのに握りが動いている"
    );
    assert_eq!(engine.console_request().await, None, "問いが残っている");
}

#[tokio::test]
async fn the_ai_cannot_answer_its_own_request() {
    // **自分で自分を許可できたら、許可の意味がありません。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);
    let _ = engine.console_take(Actor::Ai).await;

    // Act
    let refused = engine.console_answer(Actor::Ai, true).await;

    // Assert
    assert!(refused.is_err(), "AI が自分で許可できてしまう: {refused:?}");
    assert_eq!(engine.console_holder().await, None);
    assert_eq!(engine.console_request().await, Some(Actor::Ai));
}

#[tokio::test]
async fn the_human_never_has_to_ask_anyone() {
    // **人は許可を要りません**（D29 / D42）。ここは 1 ミリも変えません。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act & Assert
    assert!(engine.console_take(Actor::Human).await.is_ok());
    assert_eq!(engine.console_holder().await, Some(Actor::Human));
    assert_eq!(engine.console_request().await, None);
}
