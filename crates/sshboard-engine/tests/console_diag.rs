//! 端末（D29）が、何が起きたかを記録すること（Issue #10）。
//!
//! **サーバーを一切使いません。**ここで見たいのは「断ったことが残るか」だからです。
//!
//! 実機で、端末が入力も描画も実体と繋がらない事故が出ました。そのとき
//! `diagnostics` に残っていたのは接続の 4 行だけで、**端末の行は 1 本もありません**でした。
//!
//! > 端末を開いた・握った・失敗した、という記録が 1 行もありません。
//! > 次に踏んでも原因を追う材料が残りません。
//!
//! **そのとおりでした。**端末層が書いていたのは `console_stop` の 1 行だけです。
//! **追えない失敗は、直せない失敗**なので、ここを先に埋めます。
//!
//! 見張るのは 3 つ。
//!
//! 1. **開けなかったことが残る**（繋がっていないのに開こうとした）
//! 2. **打鍵を断ったことが残る**（開いていない端末へ打った）
//! 3. **記録に接続先が入らない**（PRD §8）

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_diag::{Level, Stage};
use sshboard_engine::{Engine, EngineError};
use sshboard_stream::OutputStream;

/// 接続一覧だけ置いた Engine。**1 件も繋いでいません。**
fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

/// 記録の全文。**どの行に何が入ったかを、そのまま失敗メッセージへ出す。**
fn rendered(engine: &Engine) -> String {
    engine
        .diagnostics()
        .recent(50)
        .iter()
        .map(|event| event.render())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn failing_to_open_the_console_is_written_down() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act — **繋がっていないのに開こうとする。**
    let refused = engine.console_open(Actor::Human, 80, 24).await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::NotConnected)),
        "断り方が違う: {refused:?}"
    );

    let events = engine.diagnostics().recent(50);
    let about_console = events
        .iter()
        .find(|event| event.stage == Stage::Exec)
        .unwrap_or_else(|| panic!("端末の記録が 1 行も無い:\n{}", rendered(&engine)));

    assert_eq!(
        about_console.level,
        Level::Error,
        "開けなかったのに失敗として残っていない:\n{}",
        rendered(&engine)
    );
    // **「駄目でした」で終わらせない**（product-baseline §17）。
    // 次の一手が無いと、人も AI も手が出ません。
    assert!(
        about_console.hint.is_some(),
        "失敗に次の一手が付いていない:\n{}",
        rendered(&engine)
    );
}

#[tokio::test]
async fn refusing_a_keystroke_is_written_down() {
    // **これが Issue #10 の症状 B を追う材料です。**
    // 画面側は握っていないと黙って捨てるので、**捨てた事実がどこにも残りません。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act — **開いていない端末へ打つ。**
    let refused = engine.console_type(Actor::Human, b"ls\n").await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::ConsoleNotOpen)),
        "断り方が違う: {refused:?}"
    );
    assert!(
        engine
            .diagnostics()
            .recent(50)
            .iter()
            .any(|event| event.stage == Stage::Exec && event.level == Level::Error),
        "打鍵を断った記録が無い:\n{}",
        rendered(&engine)
    );
}

#[tokio::test]
async fn the_console_record_never_carries_the_destination() {
    // **記録に接続先を入れない**（PRD §8）。
    // 端末の記録は、貼り付けられて相談に使われます。**そこに宛先を混ぜない。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリが作れない");
    let engine = engine_in(&dir);

    // Act
    let _ = engine.console_open(Actor::Human, 80, 24).await;
    let _ = engine.console_type(Actor::Human, b"whoami\n").await;
    engine.console_stop().await;

    // Assert
    let written = rendered(&engine);
    for leak in ["@", "://", "192.168.", "10.0.", ".com", ".local"] {
        assert!(
            !written.contains(leak),
            "記録に接続先らしきものが入っている（{leak}）:\n{written}"
        );
    }
}
