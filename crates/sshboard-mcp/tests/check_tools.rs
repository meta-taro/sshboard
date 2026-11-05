//! 点検表の MCP 側（D72）。**サーバーを一切使いません。**
//!
//! 1. **何の点検があるかを AI が引ける**（空でも行き止まりにしない）
//! 2. **知らない点検は、在るものを添えて断る**（人へ何を頼めばよいか分かる）
//! 3. **測れなかった手順は、測れずとして返る**（良しにしない）

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use sshboard_band::Band;
use sshboard_engine::Engine;
use sshboard_mcp::{CheckId, SshboardMcp};
use sshboard_stream::OutputStream;
use tokio_util::sync::CancellationToken;

fn server_in(dir: &tempfile::TempDir) -> SshboardMcp {
    let connections = dir.path().join("connections.toml");
    std::fs::write(&connections, "version = 1\n").expect("接続一覧を書けない");
    let band = Band::new();
    let stream = Arc::new(OutputStream::new());
    let engine = Engine::new(band.clone(), Arc::clone(&stream), connections);
    SshboardMcp::new(band, stream).with_engine(Arc::new(engine))
}

const CHECKS: &str = r#"
version = 1
[[check]]
id = "morning"
description = "毎朝の点検"
[[check.step]]
readonly = "disk"
label = "ディスク"
below = 90
"#;

fn id(check_id: &str) -> Parameters<CheckId> {
    Parameters(CheckId {
        check_id: check_id.to_owned(),
    })
}

#[tokio::test]
async fn the_list_is_empty_until_a_person_writes_one_and_says_how_to_add() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let server = server_in(&dir);

    let answer = server.list_checks().await.expect("空でも読める");
    let parsed: serde_json::Value = serde_json::from_str(&answer).expect("JSON");

    assert_eq!(parsed["checks"].as_array().expect("配列").len(), 0);
    assert!(parsed["howToAdd"]
        .as_str()
        .expect("書き方")
        .contains("checks.toml"));
}

#[tokio::test]
async fn the_list_shows_each_step_and_its_criteria() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(dir.path().join("checks.toml"), CHECKS).expect("書けない");
    let server = server_in(&dir);

    let answer = server.list_checks().await.expect("読める");
    let parsed: serde_json::Value = serde_json::from_str(&answer).expect("JSON");
    let morning = &parsed["checks"][0];

    assert_eq!(morning["id"], "morning");
    assert_eq!(morning["step"][0]["readonly"], "disk");
    assert_eq!(morning["step"][0]["below"], 90.0);
}

#[tokio::test]
async fn an_unknown_check_comes_back_with_the_ones_that_exist() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(dir.path().join("checks.toml"), CHECKS).expect("書けない");
    let server = server_in(&dir);

    let error = server
        .run_check(id("evening"), CancellationToken::new())
        .await
        .expect_err("無い点検が走った");

    assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    assert!(
        error.message.contains("morning"),
        "在るものを言っていない: {}",
        error.message
    );
}

#[tokio::test]
async fn a_step_that_could_not_run_is_reported_as_unknown_not_good() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(dir.path().join("checks.toml"), CHECKS).expect("書けない");
    let server = server_in(&dir);

    let answer = server
        .run_check(id("morning"), CancellationToken::new())
        .await
        .expect("繋がっていなくても報告は返る");
    let parsed: serde_json::Value = serde_json::from_str(&answer).expect("JSON");

    assert_eq!(parsed["unknown"], 1);
    assert_eq!(parsed["good"], 0);
    assert_eq!(parsed["steps"][0]["title"], "ディスク");
    assert_eq!(parsed["steps"][0]["judged"]["verdict"]["kind"], "unknown");
}
