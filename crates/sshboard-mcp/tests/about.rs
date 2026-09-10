//! **AI へ、この道具が何なのかを名乗る**（`about_sshboard`）。**サーバーへは触りません。**
//!
//! 実機の要望:
//!
//! > MCP にこのソフト概要みたいなのを AI エージェント向けに出力するやつ設置したいです。
//! > **バージョンごとになにが変わったかも返す**ように。
//!
//! ここで見張るのは 6 つ。
//!
//! 1. **版ごとに切り出せる**（見出しの形が崩れたら気づく）
//! 2. **新しい順**（AI が上から読んで、いま何ができるか分かる）
//! 3. **途中から取れる**（「前に見た版以降」だけ読める）
//! 4. **知らない版を渡されても黙って空を返さない**
//! 5. **走っている版が、この一覧に在る**（配った物と履歴がずれない）
//! 6. **接続先が 1 つも混ざらない**（ホスト名・利用者名・パス・個人名）

use sshboard_mcp::{changes_since, releases, CHANGELOG};

#[test]
fn every_release_can_be_pulled_out_on_its_own() {
    // **見出しの形が崩れたら、ここで落ちます。**
    // 崩れても人は読めますが、**AI には 1 件も届かなくなります。**
    let listed = releases(CHANGELOG);

    assert!(listed.len() >= 12, "版が少なすぎます: {}", listed.len());
    for one in &listed {
        assert!(!one.version.is_empty(), "版の番号が空: {one:?}");
        assert!(!one.headline.is_empty(), "一行が空: {one:?}");
        assert!(!one.body.trim().is_empty(), "中身が空: {one:?}");
    }
}

#[test]
fn the_newest_one_comes_first() {
    // **AI は上から読みます。**古い順だと、いま何ができるかに辿り着くのが最後になります。
    let listed = releases(CHANGELOG);

    assert_eq!(listed[0].version, "0.1.12");
    assert_eq!(listed.last().expect("空").version, "0.1.0");
}

#[test]
fn it_can_start_from_where_the_reader_left_off() {
    // **「前に見た版以降」だけ読めること。**毎回全部返すと、
    // **本当に変わった所が埋もれます。**
    let since = changes_since(CHANGELOG, Some("0.1.9")).expect("切り出せない");

    let versions: Vec<&str> = since.iter().map(|one| one.version.as_str()).collect();
    assert_eq!(
        versions,
        ["0.1.12", "0.1.11", "0.1.10"],
        "実際: {versions:?}"
    );
}

#[test]
fn asking_from_the_newest_one_gives_nothing_rather_than_everything() {
    // **最新から先は無い。**ここで全部返すと、
    // 「変わっていない」と「全部変わった」が区別できません。
    let since = changes_since(CHANGELOG, Some("0.1.12")).expect("切り出せない");

    assert!(since.is_empty(), "最新以降に何か返っている: {since:?}");
}

#[test]
fn a_version_nobody_knows_is_said_so_instead_of_returning_nothing() {
    // **黙って空を返さない**（product-baseline §8）。
    // 空だと「変わっていません」と読まれ、**打ち間違いに誰も気づけません。**
    let refused = changes_since(CHANGELOG, Some("9.9.9"));

    assert!(refused.is_none(), "知らない版を通している");
}

#[test]
fn asking_for_nothing_in_particular_gives_the_lot() {
    let all = changes_since(CHANGELOG, None).expect("切り出せない");

    assert_eq!(all.len(), releases(CHANGELOG).len());
}

#[test]
fn the_running_version_is_in_the_history() {
    // **配った物と履歴がずれない。**版を上げて履歴を書き忘れると、
    // AI は「その版で何が変わったか」を答えられません。
    let running = env!("CARGO_PKG_VERSION");
    let listed = releases(CHANGELOG);

    assert!(
        listed.iter().any(|one| one.version == running),
        "**走っている版 {running} が CHANGELOG.md に在りません。**\
         版を上げたら、同じ commit で履歴も足してください（product-baseline §10）"
    );
}

#[test]
fn no_server_of_anyone_leaks_into_it() {
    // **この文字列は AI へそのまま渡ります**（CLAUDE.md 禁止事項 4）。
    // ホスト名・IP・利用者名が 1 つでも混ざったら、そこから外へ出ます。
    for forbidden in ["@", "192.168.", "10.0.", "ssh://", "root@"] {
        assert!(
            !CHANGELOG.contains(forbidden),
            "**接続先らしきものが混ざっています**: {forbidden}"
        );
    }
}
