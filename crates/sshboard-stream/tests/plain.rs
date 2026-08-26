//! ANSI を落とす側のテスト。
//!
//! ここで守るのは Issue 005 の「MCP 側の文字列に ANSI エスケープが 1 つも混ざらない」。
//! **難しいのは境界です。**チャンクはエスケープの途中でも UTF-8 の途中でも切れる。

use sshboard_stream::PlainFilter;

/// 何も落とすものが無ければ、そのまま通る。
#[test]
fn plain_text_passes_through_unchanged() {
    // Arrange
    let mut filter = PlainFilter::new();

    // Act
    let out = filter.push(b"postfix/smtpd[1234]: connect\n");

    // Assert
    assert_eq!(out, "postfix/smtpd[1234]: connect\n");
}

#[test]
fn a_colour_sequence_is_removed() {
    // Arrange
    let mut filter = PlainFilter::new();

    // Act — 赤で "ERROR"、そのあと色を戻す
    let out = filter.push(b"\x1b[31mERROR\x1b[0m disk full\n");

    // Assert
    assert_eq!(out, "ERROR disk full\n");
}

#[test]
fn an_escape_split_across_two_chunks_is_still_removed() {
    // ネットワークから来るチャンクは、エスケープの途中で切れる。
    // Arrange
    let mut filter = PlainFilter::new();

    // Act
    let first = filter.push(b"level=\x1b[3");
    let second = filter.push(b"1mERROR\x1b[0m\n");

    // Assert
    assert_eq!(
        first, "level=",
        "途中のエスケープを出してしまっている: {first:?}"
    );
    assert_eq!(second, "ERROR\n");
}

#[test]
fn an_escape_split_one_byte_at_a_time_is_still_removed() {
    // Arrange
    let mut filter = PlainFilter::new();
    let input = b"a\x1b[1;31mb\n";

    // Act — 1 バイトずつ食わせる
    let mut out = String::new();
    for byte in input {
        out.push_str(&filter.push(&[*byte]));
    }

    // Assert
    assert_eq!(out, "ab\n");
}

#[test]
fn an_operating_system_command_is_removed() {
    // 端末のタイトルを変える並び。BEL で終わる。
    // Arrange
    let mut filter = PlainFilter::new();

    // Act
    let out = filter.push(b"\x1b]0;some title\x07done\n");

    // Assert
    assert_eq!(out, "done\n");
}

#[test]
fn an_operating_system_command_ended_by_string_terminator_is_removed() {
    // ESC \ で終わる形もある。
    let mut filter = PlainFilter::new();

    let out = filter.push(b"\x1b]0;title\x1b\\done\n");

    assert_eq!(out, "done\n");
}

#[test]
fn a_charset_designator_is_removed() {
    // ESC ( B のような中間バイト付きの並び。
    let mut filter = PlainFilter::new();

    let out = filter.push(b"\x1b(Bok\n");

    assert_eq!(out, "ok\n");
}

#[test]
fn a_multibyte_character_split_across_chunks_is_not_corrupted() {
    // UTF-8 の途中で切れても壊さない。
    // Arrange
    let mut filter = PlainFilter::new();
    let text = "接続".as_bytes();
    let (head, tail) = text.split_at(4); // 「接」の 3 バイト ＋ 「続」の 1 バイト目

    // Act
    let first = filter.push(head);
    let second = filter.push(tail);

    // Assert
    assert_eq!(
        first, "接",
        "完成していない文字を出してしまっている: {first:?}"
    );
    assert_eq!(second, "続");
}

#[test]
fn bytes_that_are_not_utf8_become_replacement_characters_instead_of_being_dropped() {
    // 対象は従来型の国内サーバー。EUC-JP / Shift_JIS が出うる（Issue 002）。
    // ここでは変換しない。**黙って消さない**ことだけを守る。
    // Arrange
    let mut filter = PlainFilter::new();

    // Act — EUC-JP の「あ」
    let out = filter.push(&[b'a', 0xA4, 0xA2, b'b', b'\n']);

    // Assert
    assert!(out.starts_with('a'), "実際: {out:?}");
    assert!(
        out.contains('\u{FFFD}'),
        "壊れたバイトを黙って消している: {out:?}"
    );
    assert!(out.ends_with("b\n"), "実際: {out:?}");
}

#[test]
fn crlf_becomes_lf() {
    // pty 経由だと \r\n で来る。AI へ渡す文字列に \r を混ぜない。
    let mut filter = PlainFilter::new();

    let out = filter.push(b"line one\r\nline two\r\n");

    assert_eq!(out, "line one\nline two\n");
}

#[test]
fn a_carriage_return_split_from_its_newline_still_becomes_one_lf() {
    // Arrange
    let mut filter = PlainFilter::new();

    // Act
    let first = filter.push(b"one\r");
    let second = filter.push(b"\ntwo");

    // Assert
    assert_eq!(first, "one", "\\r を先に出してしまっている: {first:?}");
    assert_eq!(second, "\ntwo");
}

#[test]
fn a_lone_carriage_return_is_kept() {
    // 進捗表示は \r で行頭へ戻る。\n が来ないなら握り潰さない。
    let mut filter = PlainFilter::new();

    let first = filter.push(b"50%\r");
    let second = filter.push(b"100%\n");

    assert_eq!(first, "50%");
    assert_eq!(second, "\r100%\n");
}

#[test]
fn finish_flushes_what_was_being_held() {
    // 打ち切るときに、持ち越し分を落とさない。
    let mut filter = PlainFilter::new();

    let pushed = filter.push(b"tail\r");
    let flushed = filter.finish();

    assert_eq!(pushed, "tail");
    assert_eq!(flushed, "\r");
}
