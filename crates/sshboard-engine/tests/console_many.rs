//! 端末を**複数持てる入れ物**と、**AI が 1 本ずつ読める口**（D29 の書き換え）。
//!
//! **サーバーを一切使いません。**入れ物の決まりだけを確かめます。
//! 実際に開いて読めるかは `live.rs` が実機で確かめます。
//!
//! ## なぜ入れ物から直すか
//!
//! 実機の言葉（2026-10-02）——
//!
//! > **３台同時に延長申請とか作業するから、テラターム最低２画面はいつも開いている。**
//! > しかし sshboard でやろうとしたら、**これできるのか！？から始まった**
//!
//! D29 の「1 本」が `Mutex<ConsoleSlot>` という**構造へ焼き込まれていました。**
//! 構造を N 本ぶんにして、**本数は方針（定数）へ出します。**
//! 信用の度合いが変われば定数を上げるだけで済み、作り直しになりません。
//!
//! ## 出力を 1 本にまとめない
//!
//! いまは端末も `exec` も `tail -f` も**同じ 1 本の出力**へ流れています。
//! Issue #21 がまさにそこを踏みました ——
//!
//! > `read_stream` で見分けられない
//!
//! **端末ごとに出力を持たせます。**そうでないと、端末が 2 本開いた日に
//! 「どちらが喋ったのか言えない」が確定します。

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;

/// 接続一覧だけ置いた Engine。**1 件も繋いでいません。**
fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn with_nothing_open_the_list_is_empty_not_missing() {
    // **空と「無い」を混ぜない。**`Option` で返すと、画面は
    // 「まだ聞いていない」と「1 本も無い」を区別できません。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    let open = engine.consoles().await;

    // Assert
    assert!(open.is_empty(), "開いていないのに {} 本出た", open.len());
}

#[tokio::test]
async fn asking_for_the_output_of_a_console_that_is_not_open_says_so() {
    // **空文字を返さない。**空を返すと「開いているが何も喋っていない」と
    // 区別できず、**AI は待ち続けます。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    let tail = engine.console_tail(404).await;

    // Assert
    assert!(tail.is_none(), "開いていない端末の出力が返った: {tail:?}");
}

#[tokio::test]
async fn the_shared_output_is_still_there_for_the_screen() {
    // **端末ごとに分けた結果、いまの端末の面が真っ白になる**のがいちばん怖い。
    // 共有の出力は残し、端末はそちらへも流します（画面はこれを見ています）。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    engine
        .stream()
        .push(b"hello")
        .expect("共有の出力へ流せない");

    // Assert
    assert!(
        engine.stream().plain_tail().contains("hello"),
        "共有の出力が読めない: {:?}",
        engine.stream().plain_tail()
    );
}

#[tokio::test]
async fn taking_the_console_back_still_works_with_nothing_open() {
    // **既存の振る舞いを落とさない**（`console_handover.rs` が固定しています）。
    // 入れ物を替えたときに、ここが黙って変わるのが怖い。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    engine
        .console_take(Actor::Human)
        .await
        .expect("人が握れない");

    // Assert
    assert_eq!(engine.console_holder().await, Some(Actor::Human));
    assert!(
        engine.consoles().await.is_empty(),
        "握っただけで端末が開いたことになっている"
    );
}
