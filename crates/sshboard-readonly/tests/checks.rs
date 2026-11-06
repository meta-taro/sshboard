//! 点検表（D72）。**サーバーを一切使いません。**
//!
//! 見張るのは 5 つ。
//!
//! 1. **既定は空。**人が書くまで 1 本の点検も無い
//! 2. **手順は許可リストの id を指すだけ**（任意の文字列を走らせる口を作らない・D3）
//! 3. **読めない表を、空として扱わない**（「書いたのに無い」を作らない）
//! 4. **基準は決まった形だけ**（字が出る／出ない／数が下・上）
//! 5. **測れないものを「良し」にしない**

use sshboard_readonly::{Checks, ChecksError, StepOutcome, Verdict};

const SAMPLE: &str = r#"
version = 1

[[check]]
id = "morning"
description = "毎朝の点検"

[[check.step]]
readonly = "disk-root"
label = "ルートのディスク使用率"
below = 90

[[check.step]]
readonly = "nginx-active"
label = "nginx が動いている"
contains = "active"

[[check.step]]
readonly = "dmesg-tail"
absent = "Out of memory"
"#;

fn ran(out: &str, status: Option<u32>) -> StepOutcome {
    StepOutcome::Ran {
        out: out.to_owned(),
        status,
    }
}

#[test]
fn there_is_no_check_until_a_person_writes_one() {
    let empty = Checks::empty();
    assert!(empty.is_empty());
    assert!(empty.get("morning").is_none());
}

#[test]
fn a_check_is_a_list_of_allowlisted_ids_in_the_order_written() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let morning = checks.get("morning").expect("morning が無い");

    let ids: Vec<&str> = morning
        .steps
        .iter()
        .map(|step| step.readonly.as_str())
        .collect();
    assert_eq!(ids, ["disk-root", "nginx-active", "dmesg-tail"]);
    assert_eq!(morning.description.as_deref(), Some("毎朝の点検"));
}

#[test]
fn a_step_cannot_carry_its_own_command() {
    // **手順に文字列を書く欄は無い。**書いたら読めない表として断る（D3）。
    let smuggled = r#"
version = 1
[[check]]
id = "x"
[[check.step]]
readonly = "disk-root"
run = "rm -rf /"
"#;
    assert!(matches!(
        Checks::parse(smuggled),
        Err(ChecksError::Malformed { .. })
    ));
}

#[test]
fn an_unreadable_file_is_an_error_not_an_empty_list() {
    assert!(matches!(
        Checks::parse("version = 1\n[[check]]\n"),
        Err(ChecksError::Malformed { .. })
    ));
    assert!(matches!(
        Checks::parse("version = 2\n"),
        Err(ChecksError::UnknownVersion { found: 2 })
    ));
}

#[test]
fn the_same_check_id_twice_is_refused() {
    let twice = r#"
version = 1
[[check]]
id = "a"
[[check.step]]
readonly = "x"
[[check]]
id = "a"
[[check.step]]
readonly = "y"
"#;
    assert!(matches!(
        Checks::parse(twice),
        Err(ChecksError::DuplicateId { .. })
    ));
}

#[test]
fn a_check_without_steps_is_refused() {
    let none = "version = 1\n[[check]]\nid = \"a\"\n";
    assert!(matches!(
        Checks::parse(none),
        Err(ChecksError::NoSteps { .. })
    ));
}

#[test]
fn a_missing_file_is_the_empty_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let checks =
        Checks::load_or_empty(&dir.path().join("checks.toml")).expect("無いのは異常ではない");
    assert!(checks.is_empty());
}

#[test]
fn below_and_above_compare_the_first_number_in_the_output() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let disk = &checks.get("morning").expect("morning").steps[0];

    assert_eq!(disk.judge(&ran("  42%\n", Some(0))).verdict, Verdict::Good);
    assert!(matches!(
        disk.judge(&ran("Use%\n 93%\n", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
    // **境目は悪し**（「90 未満」と書いたのだから 90 は外れ）。
    assert!(matches!(
        disk.judge(&ran("90", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
}

#[test]
fn a_number_that_cannot_be_read_is_not_good() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let disk = &checks.get("morning").expect("morning").steps[0];

    assert!(matches!(
        disk.judge(&ran("no digits here", Some(0))).verdict,
        Verdict::Unknown { .. }
    ));
}

#[test]
fn contains_and_absent_look_at_the_output() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let steps = &checks.get("morning").expect("morning").steps;

    assert_eq!(
        steps[1].judge(&ran("active\n", Some(0))).verdict,
        Verdict::Good
    );
    assert!(matches!(
        steps[1].judge(&ran("inactive\n", Some(3))).verdict,
        Verdict::Bad { .. }
    ));
    assert_eq!(
        steps[2].judge(&ran("all quiet\n", Some(0))).verdict,
        Verdict::Good
    );
    assert!(matches!(
        steps[2]
            .judge(&ran("Out of memory: Killed process 42\n", Some(0)))
            .verdict,
        Verdict::Bad { .. }
    ));
}

#[test]
fn a_step_with_no_criteria_is_good_when_the_command_exits_zero() {
    let plain = "version = 1\n[[check]]\nid = \"a\"\n[[check.step]]\nreadonly = \"uptime\"\n";
    let checks = Checks::parse(plain).expect("読めない");
    let step = &checks.get("a").expect("a").steps[0];

    assert_eq!(
        step.judge(&ran(" 10:00 up 3 days\n", Some(0))).verdict,
        Verdict::Good
    );
    assert!(matches!(
        step.judge(&ran("", Some(1))).verdict,
        Verdict::Bad { .. }
    ));
    // **終了コードが返ってこないなら、決めない。**
    assert!(matches!(
        step.judge(&ran("", None)).verdict,
        Verdict::Unknown { .. }
    ));
}

#[test]
fn a_failed_command_is_bad_even_if_the_text_matches() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let nginx = &checks.get("morning").expect("morning").steps[1];

    // `systemctl is-active` は止まっていると 3 で終わり "inactive" を出す。字だけ見ると "active" を含む。
    assert!(matches!(
        nginx.judge(&ran("inactive", Some(3))).verdict,
        Verdict::Bad { .. }
    ));
}

#[test]
fn a_step_that_did_not_run_is_unknown_and_says_why() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let disk = &checks.get("morning").expect("morning").steps[0];

    let judged = disk.judge(&StepOutcome::DidNotRun {
        why: "許可リストに在りません".to_owned(),
    });
    match judged.verdict {
        Verdict::Unknown { why } => assert!(why.contains("許可リスト")),
        other => panic!("測れなかったのに {other:?}"),
    }
}

#[test]
fn the_value_shown_is_the_first_line_trimmed_and_capped() {
    let checks = Checks::parse(SAMPLE).expect("読めない");
    let disk = &checks.get("morning").expect("morning").steps[0];

    assert_eq!(
        disk.judge(&ran("\n  42%  \nsecond\n", Some(0))).value,
        "42%"
    );
    let long = "x".repeat(500);
    assert!(disk.judge(&ran(&long, Some(0))).value.chars().count() <= 200);
}

#[test]
fn the_template_in_docs_is_a_valid_check_list() {
    // **ひな型が読めない表だと、写した人が最初に詰まる。**
    let template = include_str!("../../../docs/checks.example.toml");
    let checks = Checks::parse(template).expect("docs/checks.example.toml が読めない");
    assert_eq!(checks.get("morning").expect("morning").steps.len(), 4);
}

// --- 「最初の数」を取り違えない（コードレビュー M3・2026-11-06） ---------------------
//
// 測り違えて「良し」を出すのが、点検でいちばん困る転び方です。**数は語の頭から読み**、
// 語の途中の数字（`eth0`・`db01`）や、区切りの `-`（`web-1`）を数にしません。

fn below(limit: f64) -> sshboard_readonly::Step {
    let text = format!(
        "version = 1\n[[check]]\nid = \"a\"\n[[check.step]]\nreadonly = \"x\"\nbelow = {limit}\n"
    );
    Checks::parse(&text)
        .expect("読めない")
        .get("a")
        .expect("a")
        .steps[0]
        .clone()
}

#[test]
fn digits_inside_a_word_are_not_the_number() {
    // `eth0` の 0 を読むと、どんな値でも「90 未満」で良しになる。
    let step = below(90.0);
    assert!(matches!(
        step.judge(&ran("eth0 rx 95", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
    assert!(matches!(
        step.judge(&ran("db01: 95%", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
}

#[test]
fn a_hyphen_joining_words_is_not_a_minus_sign() {
    // `web-1: 95` の `-1` を負の数と読むと「90 未満」で良しになる。
    let step = below(90.0);
    assert!(matches!(
        step.judge(&ran("web-1: 95", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
    // 行頭や空白のあとの `-` は符号。
    assert_eq!(step.judge(&ran("-5", Some(0))).verdict, Verdict::Good);
    assert_eq!(
        step.judge(&ran("delta = -5", Some(0))).verdict,
        Verdict::Good
    );
}

#[test]
fn thousands_separators_and_exponents_are_read_whole() {
    let step = below(1000.0);
    assert!(matches!(
        step.judge(&ran("1,234 MB", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
    assert!(matches!(
        step.judge(&ran("1.5e+05", Some(0))).verdict,
        Verdict::Bad { .. }
    ));
    assert_eq!(step.judge(&ran("1.5e-05", Some(0))).verdict, Verdict::Good);
}
