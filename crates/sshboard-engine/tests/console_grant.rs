//! **期限付きの端末の許可**（2026-11-16）。**サーバーを一切使いません。**
//!
//! 時間で課金される測定用の機械へ AI が繋ぐたび、人が画面で［許可］を押すまで
//! 機械が止まったまま待っていました（6 分・30 分の実測）。
//! **人が先に「いまから N 時間は聞かずに通す」を出しておける**ようにします。
//!
//! ここで確かめるのは、緩めたぶんの**歯止め**です。
//!
//! - 許可を出せるのは**人だけ**
//! - **期限が来たら、元どおり尋ねる**
//! - **範囲の外の接続は、元どおり尋ねる**（「新しい接続」の許可は、
//!   許可を出した時点で在った接続 ＝ 人がふだん使っている接続には効かない）
//! - **人が握っている端末は奪わない**（許可があっても、人が打っている所へは入らない）
//! - 人はいつでも外せる

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError, GrantScope, MAX_CONSOLE_GRANT};
use sshboard_stream::OutputStream;
use tokio::time::Duration;

const ONE_HOUR: Duration = Duration::from_secs(60 * 60);

/// 1 件だけ載った接続一覧の中身。
fn one_connection(id: &str) -> String {
    format!(
        "\n[[connections]]\nid = \"{id}\"\nname = \"{id}\"\n\
         host = \"127.0.0.1\"\nport = 65000\nuser = \"nobody\"\n"
    )
}

fn engine_with(dir: &tempfile::TempDir, ids: &[&str]) -> (Engine, std::path::PathBuf) {
    let path = dir.path().join("connections.toml");
    let body: String = ids.iter().map(|id| one_connection(id)).collect();
    std::fs::write(&path, format!("version = 1\n{body}")).expect("接続一覧を書けない");
    let engine = Engine::new(Band::new(), Arc::new(OutputStream::new()), path.clone());
    (engine, path)
}

fn asked(result: &Result<(), EngineError>) -> bool {
    matches!(result, Err(EngineError::ConsoleApprovalNeeded))
}

#[tokio::test]
async fn a_grant_on_a_connection_lets_the_ai_hold_it_without_asking() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    let taken = engine.console_take(Actor::Ai, Some("vm")).await;

    // Assert
    assert!(taken.is_ok(), "許可があるのに断られた: {taken:?}");
    assert_eq!(engine.console_holder("vm").await, Some(Actor::Ai));
    assert_eq!(
        engine.console_request().await,
        None,
        "許可があるのに人へ問いが出ている"
    );
}

#[tokio::test]
async fn a_grant_on_one_connection_does_not_cover_another() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm", "prod"]);
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    let taken = engine.console_take(Actor::Ai, Some("prod")).await;

    // Assert
    assert!(asked(&taken), "範囲の外なのに尋ねずに通った: {taken:?}");
}

#[tokio::test(start_paused = true)]
async fn the_grant_stops_applying_once_it_expires() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    tokio::time::advance(ONE_HOUR + Duration::from_secs(1)).await;
    let taken = engine.console_take(Actor::Ai, Some("vm")).await;

    // Assert
    assert!(asked(&taken), "期限が過ぎても尋ねずに通った: {taken:?}");
    assert!(
        engine.console_grant_facts().await.is_none(),
        "期限が過ぎた許可が、まだ在ることになっている"
    );
}

#[tokio::test]
async fn the_ai_cannot_grant_itself() {
    // 自分で自分に許可を出せたら、許可の意味がありません（D42）。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);

    // Act
    let granted = engine
        .console_grant(Actor::Ai, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await;

    // Assert
    assert!(
        matches!(granted, Err(EngineError::HumanOnly)),
        "AI が自分に許可を出せた: {granted:?}"
    );
    assert!(engine.console_grant_facts().await.is_none());
}

#[tokio::test]
async fn a_grant_longer_than_the_limit_is_refused_not_shortened() {
    // **黙って縮めません。**縮めると、人は自分の出した期限を誤って覚えます。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);

    // Act
    let granted = engine
        .console_grant(
            Actor::Human,
            GrantScope::Connection("vm".into()),
            MAX_CONSOLE_GRANT + Duration::from_secs(1),
        )
        .await;

    // Assert
    assert!(
        matches!(granted, Err(EngineError::GrantTooLong { .. })),
        "上限を超える許可が通った: {granted:?}"
    );
    assert!(engine.console_grant_facts().await.is_none());
}

#[tokio::test]
async fn the_grant_does_not_take_a_console_the_human_is_holding() {
    // **人が打っている所へ、許可を理由に割り込ませません。**元どおり尋ねます。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_take(Actor::Human, Some("vm"))
        .await
        .expect("人が握れない");
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    let taken = engine.console_take(Actor::Ai, Some("vm")).await;

    // Assert
    assert!(asked(&taken), "人が握っているのに奪えた: {taken:?}");
    assert_eq!(engine.console_holder("vm").await, Some(Actor::Human));
}

#[tokio::test]
async fn a_grant_on_new_connections_covers_only_those_added_after_it() {
    // AI が測定のたびに登録する機械は、許可を出す時点ではまだ無い。
    // **その時点で在った接続（人がふだん使っている所）には効かせません。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, path) = engine_with(&dir, &["prod"]);
    engine
        .console_grant(Actor::Human, GrantScope::NewConnections, ONE_HOUR)
        .await
        .expect("人が許可を出せない");
    let mut body = std::fs::read_to_string(&path).expect("接続一覧を読めない");
    body.push_str(&one_connection("vm"));
    std::fs::write(&path, body).expect("接続一覧を書けない");

    // Act
    let on_new = engine.console_take(Actor::Ai, Some("vm")).await;
    let on_existing = engine.console_take(Actor::Ai, Some("prod")).await;

    // Assert
    assert!(on_new.is_ok(), "あとから足した接続で断られた: {on_new:?}");
    assert!(
        asked(&on_existing),
        "許可の前から在った接続に効いてしまった: {on_existing:?}"
    );
}

#[tokio::test]
async fn the_human_can_revoke_the_grant() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    engine.console_grant_revoke(Actor::Human).await;
    let taken = engine.console_take(Actor::Ai, Some("vm")).await;

    // Assert
    assert!(asked(&taken), "外したのに尋ねずに通った: {taken:?}");
    assert!(engine.console_grant_facts().await.is_none());
}

#[tokio::test]
async fn the_ai_cannot_revoke_the_grant() {
    // 外すのも人の操作です。AI が外せると、人の出した期限が黙って消えます。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_grant(Actor::Human, GrantScope::Connection("vm".into()), ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    engine.console_grant_revoke(Actor::Ai).await;

    // Assert
    assert!(engine.console_grant_facts().await.is_some());
}

#[tokio::test(start_paused = true)]
async fn the_facts_say_what_is_granted_and_how_long_is_left() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let (engine, _) = engine_with(&dir, &["vm"]);
    engine
        .console_grant(Actor::Human, GrantScope::NewConnections, ONE_HOUR)
        .await
        .expect("人が許可を出せない");

    // Act
    tokio::time::advance(Duration::from_secs(600)).await;
    let facts = engine.console_grant_facts().await.expect("許可が見えない");

    // Assert
    assert_eq!(facts.scope, GrantScope::NewConnections);
    assert_eq!(facts.remaining, ONE_HOUR - Duration::from_secs(600));
}
