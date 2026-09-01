//! 許可リスト方式の `run_readonly`（D3）。
//!
//! **サーバーを一切使いません。**ここで見たいのは「サーバーへ届く前に断るか」
//! だからです。届いてから断るのでは、**断ったことにならない。**
//!
//! 見張るのは 4 つです。
//!
//! 1. **一覧に無い識別子は、繋がっているかどうかに関わらず断る**
//! 2. **断った事実が、人の読める所に残る**（D3 追記・これが無いと一覧が育たない）
//! 3. **許可された 1 本は、断られずに接続の段階まで進む**
//! 4. **読めない一覧を、空として扱わない**（「許可したのに断られる」を作らない）

use std::path::PathBuf;
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

/// 人が書いた許可リストを置く。**製品は既定を 1 本も持ちません。**
fn allow(dir: &tempfile::TempDir, toml: &str) {
    std::fs::write(dir.path().join("readonly.toml"), toml).expect("許可リストを書けない");
}

fn refusals_at(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("readonly-refused.log")
}

#[tokio::test]
async fn an_unlisted_id_is_refused_before_it_reaches_a_server() {
    // **順番が肝です。**「繋がっていません」より先に「許可されていません」を返す。
    // 逆だと、繋がった瞬間に何でも通る作りになっていても、テストが気づけません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let error = engine
        .run_readonly(Actor::Ai, "uptime")
        .await
        .expect_err("許可していないものが通った");

    assert!(
        matches!(&error, EngineError::NotAllowed { id } if id == "uptime"),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn nothing_is_allowed_by_default() {
    // **既定は空**（D3 追記）。製品が黙って許しているコマンドは 1 本も無い。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let listed = engine.readonly_commands().expect("空の一覧は読める");

    assert!(listed.is_empty(), "既定で許しているものがある: {listed:?}");
}

#[tokio::test]
async fn the_refusal_is_left_where_the_human_can_read_it() {
    // **これが無いと許可リストは育ちません。**足りなかったものを人へ渡す唯一の経路。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let _ = engine
        .run_readonly(Actor::Ai, "systemctl-status-nginx")
        .await;

    let text = std::fs::read_to_string(refusals_at(&dir)).expect("拒否の記録が無い");
    assert!(
        text.contains("\tai\tsystemctl-status-nginx"),
        "実際: {text}"
    );
}

#[tokio::test]
async fn the_refusal_also_goes_on_the_band_so_the_person_sees_it_happen() {
    // 記録は後から読むもの。**その場で気づけるのは帯だけ。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");

    let band = Band::new();
    let mut subscriber = band.subscribe();
    let engine = Engine::new(band, Arc::new(OutputStream::new()), path);

    tokio::spawn(async move { engine.run_readonly(Actor::Ai, "uptime").await });

    let event = subscriber.recv().await.expect("帯へ出ていない");
    event.ack();

    assert_eq!(event.line().actor(), Actor::Ai);
    assert!(
        event.line().text().contains("uptime"),
        "何を断ったのか帯から分からない: {:?}",
        event.line().text()
    );
}

#[tokio::test]
async fn a_listed_command_gets_past_the_allowlist_and_stops_at_the_missing_connection() {
    // 許可されたものは断られない。**繋いでいないから止まる**のであって、
    // 許可リストで止まっているのではない。ここを混ぜると、
    // 「許可したのに動かない」の原因が分からなくなる。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(
        &dir,
        "version = 1\n\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\n",
    );
    let engine = engine_in(&dir);

    let error = engine
        .run_readonly(Actor::Ai, "uptime")
        .await
        .expect_err("繋いでいないので通らないはず");

    assert!(
        matches!(error, EngineError::NotConnected),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn passing_the_allowlist_leaves_no_refusal_behind() {
    // 断っていないものを記録に混ぜると、**人が「何が足りないか」を読めなくなる。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(
        &dir,
        "version = 1\n\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\n",
    );
    let engine = engine_in(&dir);

    let _ = engine.run_readonly(Actor::Ai, "uptime").await;

    assert!(
        !refusals_at(&dir).exists(),
        "断っていないのに記録が残っている"
    );
}

#[tokio::test]
async fn an_unreadable_allowlist_stops_instead_of_refusing_everything_in_silence() {
    // 空として扱うと、人は「許可したはずなのに断られる」に遭い、
    // **原因がファイルの書き間違いだと気づけない。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir, "これは TOML ではありません");
    let engine = engine_in(&dir);

    let error = engine
        .run_readonly(Actor::Ai, "uptime")
        .await
        .expect_err("読めない一覧を空として扱っている");

    assert!(
        matches!(error, EngineError::Allowlist(_)),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn the_engine_offers_exactly_what_the_human_listed() {
    // AI がどれを選べるかは、**人が書いた一覧そのもの。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(
        &dir,
        "version = 1\n\n\
         [[command]]\nid = \"uptime\"\nrun = \"uptime\"\ndescription = \"稼働時間\"\n",
    );
    let engine = engine_in(&dir);

    let listed = engine.readonly_commands().expect("読める");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "uptime");
    assert_eq!(listed[0].run, "uptime");
    assert_eq!(listed[0].description.as_deref(), Some("稼働時間"));
}

#[tokio::test]
async fn the_allowlist_sits_next_to_the_connection_list() {
    // **2 か所に置かない。**接続一覧と別の場所に置くと、
    // 「どちらを編集したのか」が分からなくなる（D25 で実際に食い違った）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    assert_eq!(engine.readonly_path(), dir.path().join("readonly.toml"));
}
