//! **宛先が無いまま開いている、を作らない**（Issue #8）。**サーバーを一切使いません。**
//!
//! 実機の報告:
//!
//! > ファイル画面で接続したあとディレクトリを移動すると、**接続先の表示が消え、
//! > さらに「まだサーバーに繋がっていません」の警告が出たままになります。**
//!
//! **この文言は画面のものではありませんでした。**`EngineError::NotConnected` の
//! 表示文で、`crates/sshboard-engine/src/error.rs` に在ります。画面側の
//! 「繋がっていません」（`files.notconnected`）は**別の文言**です。
//!
//! つまり**画面が古い旗を立てていたのではなく、実行体が本当に断っていた**
//! ということです。断るのは `session()` で、条件は 1 つ。
//!
//! ```text
//! held.active が指す先が held.live に無い
//! ```
//!
//! **開いているのに宛先が無い**という食い違いは、`disconnect` が
//! 「残っている 1 本へ移す」で塞いでいるはずのものです。それでも起きるなら、
//! **塞ぎ切れていない道が在る**ということになります。
//!
//! ここで見張るのは 3 つ。
//!
//! 1. **食い違ったら、直す**（開いているものが在るのに断らない）
//! 2. **直したことを記録に残す**（**黙って直すと、原因が永久に分からない**）
//! 3. **本当に 1 本も無いときは、今までどおり断る**

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_stream::OutputStream;

fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn with_nothing_open_it_still_says_so() {
    // **本当に 1 本も無いときは、今までどおり断ります。**
    // ここを緩めたら、繋がっていないのに操作が通り始めます。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.exec(Actor::Human, "true").await;

    assert!(
        matches!(refused, Err(EngineError::NotConnected)),
        "繋がっていないのに通っている: {refused:?}"
    );
    assert!(engine.active().await.is_none());
}

#[tokio::test]
async fn pointing_at_something_that_is_not_open_says_which_one() {
    // **同じ文言を 2 か所が返していました。**報告された画面写真からは、
    //
    //   - 宛先が決まっていない（`session()`）
    //   - 押したタブが開いていない（`focus()`）
    //
    // のどちらなのか**判別できませんでした。**分けたのはそのためです。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.focus("something-that-is-not-open").await;

    match refused {
        Err(EngineError::NotOpen { id }) => {
            assert_eq!(id, "something-that-is-not-open");
        }
        other => panic!("**どちらの道か分からない断り方をしている**: {other:?}"),
    }
}

#[tokio::test]
async fn refusing_a_stale_tab_leaves_something_to_read() {
    // **黙って断ると、原因が永久に分かりません。**
    // 画面の一覧が実態とずれていること自体が、報告すべき事実です。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let _ = engine.focus("stale-tab").await;

    let recorded = engine.diagnostics().recent(50);
    assert!(
        recorded
            .iter()
            .any(|line| line.message.contains("開いていない接続を宛先にしようと")),
        "記録に何も残っていない: {recorded:?}"
    );
    // **接続の識別子までしか入れません**（ホスト名は出せない・PRD §8）。
    assert!(
        recorded
            .iter()
            .any(|line| line.connection.as_deref() == Some("stale-tab")),
        "どれの話か分からない: {recorded:?}"
    );
}

#[tokio::test]
async fn nothing_open_is_not_recorded_as_a_disagreement() {
    // **繋がっていないのは異常ではありません。**毎回記録に出すと、
    // **本当の食い違いが埋もれます。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let _ = engine.exec(Actor::Human, "true").await;

    let recorded = engine.diagnostics().recent(50);
    assert!(
        !recorded.iter().any(|line| line.message.contains("宛先")),
        "繋がっていないだけで食い違いとして記録している: {recorded:?}"
    );
}
