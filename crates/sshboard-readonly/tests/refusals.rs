//! 断った事実を機械が残す（D3 追記）。
//!
//! **これが無いと許可リストは育ちません。**
//! 「何が足りなかったか」を人が推測する羽目になり、結局は勘で足すことになる。
//!
//! 残すのは 3 列だけです。**接続先も引数も残しません**（PRD §8）。

use sshboard_band::Actor;
use sshboard_readonly::Refusals;

#[test]
fn a_refusal_is_appended_without_erasing_the_line_before_it() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("readonly-refused.log");
    let refusals = Refusals::at(&path);

    refusals.record(Actor::Ai, "first").expect("1 行目を残せる");
    refusals
        .record(Actor::Ai, "second")
        .expect("2 行目を残せる");

    let text = std::fs::read_to_string(&path).expect("読める");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "追記のはずが上書きされている: {text}");
    assert!(lines[0].ends_with("\tfirst"));
    assert!(lines[1].ends_with("\tsecond"));
}

#[test]
fn a_refusal_line_is_three_tab_separated_columns() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("readonly-refused.log");

    Refusals::at(&path)
        .record(Actor::Ai, "uptime")
        .expect("残せる");

    let text = std::fs::read_to_string(&path).expect("読める");
    let columns: Vec<&str> = text.trim_end().split('\t').collect();

    assert_eq!(columns.len(), 3, "列が 3 つではない: {text:?}");
    // 時刻は RFC 3339 の UTC。**手元の時間帯を残さない。**
    assert!(
        columns[0].ends_with('Z'),
        "時刻が UTC ではない: {:?}",
        columns[0]
    );
    assert_eq!(columns[1], "ai");
    assert_eq!(columns[2], "uptime");
}

#[test]
fn a_refusal_line_never_carries_a_newline_from_the_requested_id() {
    // 呼ぶ側の文字列がそのまま行に入ると、**1 件の拒否が 2 行に見える。**
    // 記録を数えて許可リストを育てる以上、件数が狂うのは困る。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("readonly-refused.log");

    Refusals::at(&path)
        .record(Actor::Ai, "one\nrm -rf /\ttwo")
        .expect("残せる");

    let text = std::fs::read_to_string(&path).expect("読める");
    assert_eq!(
        text.lines().count(),
        1,
        "1 件が複数行になっている: {text:?}"
    );
    assert_eq!(
        text.trim_end().split('\t').count(),
        3,
        "列が増えている: {text:?}"
    );
}

#[test]
fn the_folder_is_created_when_it_is_not_there_yet() {
    // 設定ディレクトリがまだ無い機械でも、**最初の拒否を落とさない。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("not-yet").join("readonly-refused.log");

    Refusals::at(&path)
        .record(Actor::Ai, "uptime")
        .expect("親ごと作って残せる");

    assert!(path.exists());
}

#[test]
fn the_human_side_is_recorded_as_human() {
    // 人は許可リストに縛られません（D3）。それでも記録の形は 1 つにしておく。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("readonly-refused.log");

    Refusals::at(&path)
        .record(Actor::Human, "uptime")
        .expect("残せる");

    let text = std::fs::read_to_string(&path).expect("読める");
    assert!(text.contains("\thuman\t"), "{text:?}");
}
