//! 記録の性質。**溜めすぎない・順序が読める・失敗には必ず次の一手が付く。**

use sshboard_diag::{Diagnostics, Level, Stage};

#[test]
fn the_newest_events_come_first_because_that_is_what_people_read() {
    // Arrange
    let diag = Diagnostics::new();

    // Act
    diag.info(Stage::Reach, Some("app"), "1 本目");
    diag.info(Stage::Reach, Some("app"), "2 本目");
    diag.info(Stage::Reach, Some("app"), "3 本目");

    // Assert
    let recent = diag.recent(2);
    assert_eq!(recent.len(), 2, "件数の上限が効いていない");
    assert_eq!(recent[0].message, "3 本目", "新しい順になっていない");
    assert_eq!(recent[1].message, "2 本目");
}

#[test]
fn the_sequence_number_keeps_rising_even_after_old_events_are_dropped() {
    // 通し番号が振り直されると、**取りこぼしたことに気づけない。**
    let diag = Diagnostics::with_capacity(3);
    for index in 0..10 {
        diag.info(Stage::Auth, None, format!("{index}"));
    }

    let recent = diag.recent(3);
    assert_eq!(recent[0].seq, 9);
    assert_eq!(recent[2].seq, 7);
}

#[test]
fn old_events_are_dropped_but_the_count_is_not_hidden() {
    // **黙って消さない。**「全部見えている」と誤解させると、
    // 見えていない失敗を「起きていない」と読んでしまう。
    let diag = Diagnostics::with_capacity(2);

    diag.info(Stage::Reach, None, "a");
    diag.info(Stage::Reach, None, "b");
    assert_eq!(diag.dropped(), 0);

    diag.info(Stage::Reach, None, "c");
    assert_eq!(diag.len(), 2, "上限を超えて溜めている");
    assert_eq!(diag.dropped(), 1, "捨てたことを数えていない");
}

#[test]
fn a_capacity_of_zero_still_keeps_one_event() {
    // 0 件だと記録の意味が消える。**設定ミスで無音にしない。**
    let diag = Diagnostics::with_capacity(0);
    diag.info(Stage::Auth, None, "残る");

    assert_eq!(diag.len(), 1);
}

#[test]
fn a_failure_always_carries_what_to_do_next() {
    // 「駄目でした」で終わらせない（product-baseline §17）。
    let diag = Diagnostics::new();

    diag.error(
        Stage::Auth,
        Some("app"),
        "ssh-agent の鍵 1 本とも受け付けられませんでした",
        "ssh-add で対応する鍵を入れるか、接続に鍵のパスを設定してください",
    );

    let event = &diag.recent(1)[0];
    assert_eq!(event.level, Level::Error);
    assert!(event.hint.is_some(), "失敗なのに次の一手が無い");
}

#[test]
fn events_carry_elapsed_time_not_a_wall_clock() {
    // **時刻を持たない。**記録を貼ったときに、その人がいつ何をしていたかまで
    // 分かる必要はない。知りたいのは前後関係と所要時間。
    let diag = Diagnostics::new();
    diag.info(Stage::Reach, None, "始め");

    let rendered = serde_json::to_string(&diag.recent(1)[0]).expect("JSON にできない");
    assert!(rendered.contains("\"atMs\""), "経過時間が無い: {rendered}");
    assert!(!rendered.contains("2026"), "実時刻が入っている: {rendered}");
}

#[test]
fn the_rendered_line_shows_the_stage_and_the_next_step() {
    let diag = Diagnostics::new();
    diag.error(
        Stage::HostKey,
        Some("app"),
        "初めて見るホストです",
        "指紋を確かめて登録してください",
    );

    let line = diag.recent(1)[0].render();
    assert!(line.contains("失敗"), "深刻さが読めない: {line}");
    assert!(line.contains("ホスト鍵"), "段階が読めない: {line}");
    assert!(line.contains("[app]"), "どの接続か読めない: {line}");
    assert!(line.contains("→"), "次の一手が読めない: {line}");
}

#[test]
fn clones_share_one_log_rather_than_each_keeping_their_own() {
    // GUI と MCP が別々の記録を見ると、**片方にしか出ない失敗**が生まれる。
    let diag = Diagnostics::new();
    let elsewhere = diag.clone();

    elsewhere.info(Stage::Mcp, None, "AI から");

    assert_eq!(diag.len(), 1, "複製が別の記録を持っている");
}
