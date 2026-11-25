//! 帯の控え（Phase 2「監査ログの永続化」・2026-11-25）。
//!
//! 帯は画面の中にしか無く、アプリを閉じると消えていました。
//! **「昨日 AI は何をしたか」を後から人が確かめられない**のは、記録として欠けています。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use sshboard_band::{Actor, BandLine, Journal};

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
/// 2026-11-25T00:00:00Z
const NOV_25: u64 = 1_795_564_800_000;

fn fresh_dir() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sshboard-journal-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_line_is_kept_under_the_day_it_happened_and_reads_back_in_order() {
    let dir = fresh_dir();
    let journal = Journal::new(&dir, 14);

    journal
        .append(NOV_25 + 1_000, &BandLine::new(0, Actor::Ai, "connect web"))
        .unwrap();
    journal
        .append(
            NOV_25 + 2_000,
            &BandLine::new(1, Actor::Human, "rm /srv/a \"b\"\n"),
        )
        .unwrap();

    assert_eq!(journal.days().unwrap(), vec!["2026-11-25".to_owned()]);
    let lines = journal.read_day("2026-11-25").unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].at_ms, NOV_25 + 1_000);
    assert_eq!(lines[0].tag, "[AI]");
    assert_eq!(lines[0].text, "connect web");
    // **引用符や改行が入っても 1 行 1 件が崩れない。**
    assert_eq!(lines[1].tag, "[Human]");
    assert_eq!(lines[1].text, "rm /srv/a \"b\"\n");
}

#[test]
fn days_are_listed_newest_first() {
    let dir = fresh_dir();
    let journal = Journal::new(&dir, 14);
    journal
        .append(NOV_25 - DAY_MS, &BandLine::new(0, Actor::Ai, "a"))
        .unwrap();
    journal
        .append(NOV_25, &BandLine::new(1, Actor::Ai, "b"))
        .unwrap();

    assert_eq!(
        journal.days().unwrap(),
        vec!["2026-11-25".to_owned(), "2026-11-24".to_owned()]
    );
}

#[test]
fn days_older_than_the_keep_window_are_removed_and_nothing_else_is_touched() {
    let dir = fresh_dir();
    let journal = Journal::new(&dir, 14);
    journal
        .append(NOV_25 - 15 * DAY_MS, &BandLine::new(0, Actor::Ai, "old"))
        .unwrap();
    journal
        .append(NOV_25 - 13 * DAY_MS, &BandLine::new(1, Actor::Ai, "kept"))
        .unwrap();
    // **控えでないファイルには触らない。**同じ場所に人が何か置いても消さない。
    std::fs::write(dir.join("notes.txt"), "mine").unwrap();

    journal.prune(NOV_25).unwrap();

    assert_eq!(journal.days().unwrap(), vec!["2026-11-12".to_owned()]);
    assert!(dir.join("notes.txt").exists());
}

#[test]
fn a_day_that_is_not_a_date_is_refused_instead_of_opening_another_file() {
    let dir = fresh_dir();
    let journal = Journal::new(&dir, 14);
    for bad in ["../connections", "2026-11-25/../../x", "", "2026-1-5"] {
        assert!(journal.read_day(bad).is_err(), "{bad} を開いてしまった");
    }
}

#[test]
fn an_empty_or_missing_folder_has_no_days() {
    let journal = Journal::new(fresh_dir(), 14);
    assert!(journal.days().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn the_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir();
    let journal = Journal::new(&dir, 14);
    journal
        .append(NOV_25, &BandLine::new(0, Actor::Ai, "x"))
        .unwrap();
    let mode = std::fs::metadata(dir.join("band-2026-11-25.jsonl"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o077, 0, "ほかの利用者に読めます: {mode:o}");
}
