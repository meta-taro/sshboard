//! 状態を変える操作の関門（D45 / D47 / Issue #17）。**サーバーを一切使いません。**
//!
//! ここで見たいのは「**走る前に止まるか**」だからです。走ってから止めるのでは、
//! 止めたことになりません。
//!
//! > **「人が見ている」を安全の根拠にしているのを、「人が承認した」に移す**
//!
//! 見張るのは 7 つ。
//!
//! 1. **既定は空。**人が書くまで 1 本も走らない
//! 2. **AI は、人が許可するまで走らせられない**
//! 3. **問いには、実際に打つものが載る**（何を許すのか読めないと答えられない）
//! 4. **AI は自分で自分を許可できない**
//! 5. **断ったら走らない**
//! 6. **許可は使い切り。**1 回の許可で 1 回だけ
//! 7. **人はいつでも走らせられる**（承認も上限もかからない）

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

/// 人が書いた一覧を置く。**製品は既定を 1 本も持ちません。**
fn allow(dir: &tempfile::TempDir) {
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"reload-httpd\"\n\
         run = \"systemctl reload httpd\"\n\
         description = \"設定を読み直させる（無停止）\"\nmax_per_hour = 2\n",
    )
    .expect("一覧を書けない");
}

#[tokio::test]
async fn nothing_runs_until_a_person_writes_the_file() {
    // **既定は空**（D3 と同じ立場）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(refused, Err(EngineError::NotAllowed { .. })),
        "一覧が無いのに通っている: {refused:?}"
    );
}

#[tokio::test]
async fn the_ai_cannot_run_one_until_the_person_allows_it() {
    // **これが芯です。**書いてあるだけでは走りません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(refused, Err(EngineError::ConsoleApprovalNeeded)),
        "**承認なしで走ろうとしている**: {refused:?}"
    );
}

#[tokio::test]
async fn the_question_carries_what_would_actually_run() {
    // **何が走るのか読めないと、人は答えられません。**
    // 「restart-httpd を許可しますか」だけでは、実際に何が打たれるか分かりません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let asked = engine.operation_request().expect("問いが立っていない");
    assert_eq!(asked.0, "reload-httpd");
    assert_eq!(asked.1, "systemctl reload httpd");
}

#[tokio::test]
async fn the_ai_cannot_answer_its_own_request() {
    // **自分で自分を許可できたら、承認の意味がありません。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let refused = engine.answer_operation(Actor::Ai, true).await;

    assert!(refused.is_err(), "AI が自分で許可できてしまう: {refused:?}");
    assert!(engine.operation_request().is_some(), "問いが消えている");
}

#[tokio::test]
async fn refusing_leaves_the_operation_unrunnable() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    engine
        .answer_operation(Actor::Human, false)
        .await
        .expect("人が断れない");

    assert!(engine.operation_request().is_none(), "問いが残っている");
    // 断ったのだから、次に呼んでも走らない（また尋ねる）。
    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "断ったのに走ろうとしている: {again:?}"
    );
}

#[tokio::test]
async fn an_approval_is_spent_once() {
    // **使い切り。**1 回許したら 1 回だけ。
    // 許しっぱなしにすると、**人が見ていない間に何度でも走ります。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;
    engine
        .answer_operation(Actor::Human, true)
        .await
        .expect("人が許せない");

    // 1 回目は許可を使う（繋がっていないので、そこで止まるのが正しい）。
    let used = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(used, Err(EngineError::NotConnected)),
        "許可が使われていない: {used:?}"
    );

    // 2 回目は、また尋ねる。
    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "**1 回の許可で 2 回走ろうとしている**: {again:?}"
    );
}

#[tokio::test]
async fn a_person_never_has_to_ask_anyone() {
    // **人は承認も上限も要りません。**画面の前に居るのだから、見えています。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let ran = engine.run_operation(Actor::Human, "reload-httpd").await;

    // 繋がっていないので、そこで止まるのが正しい。**承認では止まらない。**
    assert!(
        matches!(ran, Err(EngineError::NotConnected)),
        "人が承認を求められている: {ran:?}"
    );
    assert!(engine.operation_request().is_none(), "人に問いが立っている");
}
