//! 点検表を回す（D72）。**サーバーを一切使いません。**
//!
//! 走らせた値の良し悪しは `live.rs` の側（テスト用サーバー）で見ます。ここで見るのは、
//!
//! 1. **表は接続一覧の隣**（2 か所に置かない）
//! 2. **知らない点検は、在るものを添えて断る**
//! 3. **読めない表は、空として扱わない**
//! 4. **測れなかった手順は「測れず」で、理由が付く**（良しにしない）
//! 5. **途中で測れなくても、最後まで回す**

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_readonly::Verdict;
use sshboard_stream::OutputStream;

fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

fn write(dir: &tempfile::TempDir, name: &str, toml: &str) {
    std::fs::write(dir.path().join(name), toml).expect("書けない");
}

const CHECKS: &str = r#"
version = 1

[[check]]
id = "morning"
description = "毎朝の点検"

[[check.step]]
readonly = "uptime"

[[check.step]]
readonly = "not-listed"
label = "許可リストに無いもの"
"#;

#[test]
fn the_check_list_sits_next_to_the_connection_list() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    assert_eq!(engine.checks_path(), dir.path().join("checks.toml"));
}

#[test]
fn there_is_no_check_until_a_person_writes_one() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    assert!(engine.checks().expect("無いのは異常ではない").is_empty());
}

#[tokio::test]
async fn an_unknown_check_is_refused_with_the_ones_that_exist() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "checks.toml", CHECKS);
    let engine = engine_in(&dir);

    let error = engine
        .run_check(Actor::Ai, "evening")
        .await
        .expect_err("無い点検が走った");

    match &error {
        EngineError::UnknownCheck { id, have, .. } => {
            assert_eq!(id, "evening");
            assert_eq!(have, &["morning".to_owned()]);
        }
        other => panic!("実際: {other:?}"),
    }
    assert!(
        error.to_string().contains("morning"),
        "在るものを言っていない: {error}"
    );
}

#[tokio::test]
async fn an_unreadable_check_list_stops_instead_of_looking_empty() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "checks.toml", "version = 1\n[[check]]\nid = \"a\"\n");
    let engine = engine_in(&dir);

    let error = engine
        .run_check(Actor::Ai, "a")
        .await
        .expect_err("読めない表で走った");
    assert!(matches!(error, EngineError::Checks(_)), "実際: {error:?}");
}

#[tokio::test]
async fn every_step_is_tried_and_one_that_could_not_run_says_why() {
    // 繋がっていない。**1 つ目で止めずに 2 つ目まで回し、どちらも「測れず」。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "checks.toml", CHECKS);
    write(
        &dir,
        "readonly.toml",
        "version = 1\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\n",
    );
    let engine = engine_in(&dir);

    let report = engine
        .run_check(Actor::Ai, "morning")
        .await
        .expect("回せない");

    assert_eq!(report.steps.len(), 2, "途中で止まった");
    assert_eq!(report.unknown, 2);
    assert_eq!(report.good + report.bad, 0);
    assert_eq!(report.steps[1].title, "許可リストに無いもの");
    for step in &report.steps {
        match &step.judged.verdict {
            Verdict::Unknown { why } => assert!(!why.is_empty(), "理由が空"),
            other => panic!("測れなかったのに {other:?}"),
        }
    }
}

#[tokio::test]
async fn the_last_run_can_be_read_back_by_the_screen() {
    // **AI が回した点検を、人も画面で見られる**（D72）。回すのはどちらでも、残る所は 1 つ。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "checks.toml", CHECKS);
    let engine = engine_in(&dir);

    assert!(
        engine.last_check("morning").await.is_none(),
        "回す前から結果がある"
    );
    let ran = engine
        .run_check(Actor::Ai, "morning")
        .await
        .expect("回せない");

    assert_eq!(engine.last_check("morning").await, Some(ran));
}
