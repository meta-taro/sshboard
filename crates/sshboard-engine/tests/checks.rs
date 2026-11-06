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

// --- AI が点検の案を出し、画面の人が保存する（D72 の次の段） ---------------------------

const ALLOWED: &str = "version = 1\n[[command]]\nid = \"disk\"\nrun = \"df\"\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\n";

fn proposed(id: &str, readonly: &str) -> sshboard_readonly::Check {
    let text = format!(
        "version = 1\n[[check]]\nid = \"{id}\"\ndescription = \"案\"\n[[check.step]]\nreadonly = \"{readonly}\"\nbelow = 90\n"
    );
    sshboard_readonly::Checks::parse(&text)
        .expect("案が読めない")
        .get(id)
        .expect("案が無い")
        .clone()
}

fn engine_with_screen(dir: &tempfile::TempDir) -> Engine {
    let band = Band::new();
    let mut seen = band.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = seen.recv().await {
            event.ack();
        }
    });
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(band, Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn a_proposal_naming_a_command_that_is_not_allowed_is_refused_before_the_person_sees_it() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);

    let refused = engine
        .propose_check(Actor::Ai, proposed("daily", "rm-everything"))
        .await;

    match refused {
        Err(EngineError::BadArgument(why)) => {
            assert!(why.contains("disk"), "在るものを言っていない: {why}")
        }
        other => panic!("通った: {other:?}"),
    }
    assert!(engine.check_proposal().is_none(), "断ったのに画面へ出した");
}

#[tokio::test]
async fn a_proposal_waits_for_the_person_and_only_one_waits_at_a_time() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);

    engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await
        .expect("出せない");
    assert_eq!(
        engine.check_proposal().map(|check| check.id),
        Some("daily".to_owned())
    );

    let second = engine
        .propose_check(Actor::Ai, proposed("weekly", "uptime"))
        .await;
    assert!(
        matches!(second, Err(EngineError::ProposalPending { .. })),
        "上書きした: {second:?}"
    );
    // **同じ id の出し直しは通す**（直した版を見せる）。
    engine
        .propose_check(Actor::Ai, proposed("daily", "uptime"))
        .await
        .expect("出し直せない");
}

#[tokio::test]
async fn saving_writes_a_check_list_that_reads_back_and_clears_the_proposal() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    write(&dir, "checks.toml", "version = 1\n\n# 人が書いた注記\n[[check]]\nid = \"old\"\n[[check.step]]\nreadonly = \"uptime\"\n");
    let engine = engine_with_screen(&dir);
    engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await
        .expect("出せない");

    let shown = engine.check_proposal().expect("案が出ていない");
    let saved = engine.save_check_proposal(&shown).expect("保存できない");

    assert_eq!(saved.id, "daily");
    assert!(
        engine.check_proposal().is_none(),
        "保存したのに案が残っている"
    );
    let text = std::fs::read_to_string(dir.path().join("checks.toml")).expect("読めない");
    assert!(
        text.contains("# 人が書いた注記"),
        "人の書いたものを消した: {text}"
    );
    let checks = engine.checks().expect("保存した表が読めない");
    assert!(
        checks.get("old").is_some() && checks.get("daily").is_some(),
        "{text}"
    );
}

#[tokio::test]
async fn a_check_that_already_exists_is_not_proposed_over() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    write(
        &dir,
        "checks.toml",
        "version = 1\n[[check]]\nid = \"daily\"\n[[check.step]]\nreadonly = \"uptime\"\n",
    );
    let engine = engine_with_screen(&dir);

    let refused = engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await;
    assert!(
        matches!(refused, Err(EngineError::BadArgument(_))),
        "上書きの案が通った: {refused:?}"
    );
}

#[tokio::test]
async fn declining_writes_nothing() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);
    engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await
        .expect("出せない");

    engine.dismiss_check_proposal();

    assert!(engine.check_proposal().is_none());
    assert!(!dir.path().join("checks.toml").exists(), "断ったのに書いた");
}

// --- レビューの指摘（2026-11-06） --------------------------------------------------------

#[tokio::test]
async fn a_proposal_swapped_after_the_person_saw_it_is_not_saved() {
    // **人が見た案だけを保存する**（セキュリティレビュー MEDIUM-1）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);
    engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await
        .expect("出せない");
    let seen = engine.check_proposal().expect("案が出ていない");
    // 押す直前に、AI が同じ id で手順の違う案に差し替える。
    engine
        .propose_check(Actor::Ai, proposed("daily", "uptime"))
        .await
        .expect("出し直せない");

    let refused = engine.save_check_proposal(&seen);

    assert!(
        matches!(refused, Err(EngineError::BadArgument(_))),
        "見ていない案を保存した: {refused:?}"
    );
    assert!(!dir.path().join("checks.toml").exists());
    assert_eq!(
        engine
            .check_proposal()
            .map(|check| check.steps[0].readonly.clone()),
        Some("uptime".to_owned())
    );
}

#[tokio::test]
async fn an_oversized_proposal_is_refused_before_the_person_sees_it() {
    // **画面を埋める案で、大事な手順を見落とさせない**（セキュリティレビュー LOW-1）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);

    let mut many = proposed("daily", "disk");
    many.steps = vec![many.steps[0].clone(); 21];
    assert!(matches!(
        engine.propose_check(Actor::Ai, many).await,
        Err(EngineError::BadArgument(_))
    ));

    let mut wordy = proposed("daily", "disk");
    wordy.description = Some("x".repeat(500));
    assert!(matches!(
        engine.propose_check(Actor::Ai, wordy).await,
        Err(EngineError::BadArgument(_))
    ));
    assert!(engine.check_proposal().is_none());
}

#[tokio::test]
async fn saving_leaves_no_half_written_file_behind() {
    // **書いている途中で落ちても、人の育てた表を空にしない**（コードレビュー L1）。一時ファイルに書いて置き換える。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    write(&dir, "readonly.toml", ALLOWED);
    let engine = engine_with_screen(&dir);
    engine
        .propose_check(Actor::Ai, proposed("daily", "disk"))
        .await
        .expect("出せない");
    let shown = engine.check_proposal().expect("案");

    engine.save_check_proposal(&shown).expect("保存できない");

    let left: Vec<String> = std::fs::read_dir(dir.path())
        .expect("読めない")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("checks") && name != "checks.toml")
        .collect();
    assert!(left.is_empty(), "書きかけが残っている: {left:?}");
}

#[test]
fn a_run_stops_measuring_once_the_target_has_moved() {
    // **回している途中で宛先が変わったら、残りは測らない**（コードレビュー M2）。
    // 前半は A・後半は B の値が A の記録に混ざるのを防ぐ。
    use sshboard_engine::moved_since;
    assert!(moved_since(Some("a"), Some("a")).is_none());
    let why = moved_since(Some("a"), Some("b")).expect("変わったのに測った");
    assert!(why.contains("a") && why.contains("b"), "{why}");
    assert!(moved_since(Some("a"), None).is_some(), "切れたのに測った");
}
