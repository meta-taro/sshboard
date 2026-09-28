//! 外から書き換えられた一覧を拾えるか。
//!
//! **なぜ要るか**: `ConnectionsWatch` が鳴るのは、**この中で書き換えたとき**だけだった。
//! 画面が保存した・MCP が書き換えた、は鳴る。**人がエディタで書き換えた、は鳴らない。**
//! 2026-09-28 に指摘されて分かった（D59）。

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sshboard_connections::{look_once, ConnectionsWatch, FileStamp, Look};

fn one(id: &str) -> String {
    format!(
        "version = 1\n\n[[connections]]\nid = \"{id}\"\nname = \"{id}\"\nhost = \"example.invalid\"\nuser = \"ops\"\n"
    )
}

fn put(path: &Path, text: &str) {
    std::fs::write(path, text).expect("書けない");
}

#[test]
fn an_untouched_file_is_not_announced() {
    // Arrange
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let mut seen = FileStamp::of(&path);

    // Act & Assert
    assert_eq!(look_once(&path, &mut seen), Look::Same);
    assert_eq!(look_once(&path, &mut seen), Look::Same, "2 周目で鳴った");
}

#[test]
fn a_file_rewritten_from_outside_is_announced() {
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let mut seen = FileStamp::of(&path);

    put(&path, &one("two"));

    assert_eq!(look_once(&path, &mut seen), Look::Changed);
}

#[test]
fn a_rewrite_of_the_very_same_length_is_still_announced() {
    // **mtime と大きさだけを見ていると、ここを取り落とす。**
    // `id` を 1 文字だけ変えた書き換えは、長さが変わらない。
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("aaa"));
    let mut seen = FileStamp::of(&path);

    put(&path, &one("aab"));

    assert_eq!(look_once(&path, &mut seen), Look::Changed);
}

#[test]
fn deleting_the_file_is_announced_too() {
    // 一覧が空になった、も状態の変化。**黙って古い一覧を出し続けない。**
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let mut seen = FileStamp::of(&path);

    std::fs::remove_file(&path).expect("消せない");

    assert_eq!(look_once(&path, &mut seen), Look::Changed);
}

#[test]
fn a_half_written_file_is_not_announced() {
    // エディタが保存の最中のものを拾うと、**画面に「読めません」の帯が一瞬出る。**
    // 書き途中は「新しい状態」ではない。
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let mut seen = FileStamp::of(&path);

    put(&path, "version = 1\n\n[[connections]]\nid = \"half");

    assert_eq!(look_once(&path, &mut seen), Look::Unreadable);
}

#[test]
fn an_unreadable_file_is_complained_about_once_not_every_round() {
    // 2 秒ごとに同じ不満を出し続けない。
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, "version = 1\n[[connections\n");
    let mut seen = FileStamp::of(&path);

    // 印は「壊れている今の姿」で始まるので、以後は Same。
    assert_eq!(look_once(&path, &mut seen), Look::Same);

    put(&path, "version = 1\n[[connections\nid = \"half\"\n");
    assert_eq!(look_once(&path, &mut seen), Look::Unreadable);
    assert_eq!(
        look_once(&path, &mut seen),
        Look::Same,
        "同じ壊れたファイルで毎周鳴っている"
    );
}

#[tokio::test]
async fn an_outside_rewrite_reaches_whoever_is_watching() {
    // ここが本番の形。**見張りが実際に鳴るか。**
    // Arrange
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let watch = Arc::new(ConnectionsWatch::new());
    let mut watcher = watch.subscribe();
    let looking = tokio::spawn(sshboard_connections::look_for_external_changes(
        path.clone(),
        Arc::clone(&watch),
    ));

    // Act — 見張りが始まるのを待ってから、外から書き換える。
    tokio::time::sleep(Duration::from_millis(50)).await;
    put(&path, &one("two"));

    // Assert
    let heard = tokio::time::timeout(Duration::from_secs(10), watcher.recv()).await;
    looking.abort();
    assert!(
        heard.is_ok(),
        "外からの書き換えが届いていない（10 秒待った）"
    );
    assert!(heard.expect("時間切れ").is_ok(), "口が閉じている");
}

#[tokio::test]
async fn nothing_is_announced_while_the_file_sits_still() {
    // **鳴り続ける見張りは、鳴らない見張りと同じくらい悪い。**
    let dir = tempfile::tempdir().expect("作れない");
    let path = dir.path().join("connections.toml");
    put(&path, &one("one"));
    let watch = Arc::new(ConnectionsWatch::new());
    let mut watcher = watch.subscribe();
    let looking = tokio::spawn(sshboard_connections::look_for_external_changes(
        path.clone(),
        Arc::clone(&watch),
    ));

    let heard = tokio::time::timeout(Duration::from_secs(6), watcher.recv()).await;
    looking.abort();
    assert!(heard.is_err(), "誰も触っていないのに鳴った");
}
