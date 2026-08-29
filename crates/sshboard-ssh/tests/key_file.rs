//! 鍵ファイルの形式を、**中身だけで**見分ける。
//!
//! **拡張子を信用しません。**実物で裏切られました — `*.tera.ppk` という名前の
//! ファイルの中身が OpenSSH 秘密鍵で、拡張子で判定していた製品は
//! 「PuTTY 形式です、変換してください」と、**要らない作業へ人を送っていました**（D28）。
//!
//! ここに本物の鍵は 1 つも置きません。**見出しだけで判定できる**設計にしてあります。

use sshboard_ssh::{inspect_key, KeyFormat, KeyVerdict};

/// PPK の見出しだけを組む。**鍵の中身は入っていません。**
fn ppk_header(version: u8, encryption: &str) -> String {
    format!(
        "PuTTY-User-Key-File-{version}: ssh-ed25519\r\n\
         Encryption: {encryption}\r\n\
         Comment: sshboard-test\r\n"
    )
}

/// OpenSSH の見出しだけを組む。
///
/// 本文の先頭は base64 で `openssh-key-v1\0` ＋ 暗号方式名です。
/// **`none` の鍵だけが、この決まった前置きになります。**
fn openssh_header(encrypted: bool) -> String {
    let body = if encrypted {
        "b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0"
    } else {
        "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAAB"
    };
    format!("-----BEGIN OPENSSH PRIVATE KEY-----\n{body}\n-----END OPENSSH PRIVATE KEY-----\n")
}

#[test]
fn a_putty_key_is_recognised_by_its_content() {
    for (version, expected) in [(2, KeyFormat::Ppk2), (3, KeyFormat::Ppk3)] {
        let facts = inspect_key(ppk_header(version, "aes256-cbc").as_bytes());
        assert_eq!(facts.format, expected);
        assert!(
            facts.needs_passphrase,
            "暗号化された PPK を素の鍵と見ている"
        );
        assert!(facts.usable(), "russh は PPK を読めます（v2 / v3 とも）");
    }
}

#[test]
fn a_putty_key_without_a_passphrase_is_not_asked_for_one() {
    // 余計に聞くのも害です。**人は「何を入れればいいのか」で止まります。**
    let facts = inspect_key(ppk_header(3, "none").as_bytes());
    assert_eq!(facts.format, KeyFormat::Ppk3);
    assert!(!facts.needs_passphrase);
}

#[test]
fn an_openssh_key_is_recognised_whatever_the_file_is_called() {
    // **実物がこれでした。**`*.tera.ppk` の中身が OpenSSH 秘密鍵。
    let facts = inspect_key(openssh_header(true).as_bytes());
    assert_eq!(facts.format, KeyFormat::OpenSsh);
    assert!(facts.needs_passphrase);

    let plain = inspect_key(openssh_header(false).as_bytes());
    assert_eq!(plain.format, KeyFormat::OpenSsh);
    assert!(!plain.needs_passphrase, "素の鍵にパスフレーズを聞いている");
}

#[test]
fn the_older_pem_shapes_are_recognised_too() {
    // 古い環境には PKCS#1 / PKCS#8 が残っています（Issue 002 で実機に在った）。
    let pkcs1_encrypted = "-----BEGIN RSA PRIVATE KEY-----\n\
                           Proc-Type: 4,ENCRYPTED\n\
                           DEK-Info: AES-128-CBC,0123456789ABCDEF\n";
    let facts = inspect_key(pkcs1_encrypted.as_bytes());
    assert_eq!(facts.format, KeyFormat::Pkcs1);
    assert!(facts.needs_passphrase);

    // 素の PKCS#8 は読める。
    let plain = inspect_key(b"-----BEGIN PRIVATE KEY-----\n");
    assert_eq!(plain.format, KeyFormat::Pkcs8);
    assert!(plain.usable());
    assert!(!plain.needs_passphrase);
}

#[test]
fn a_shape_russh_cannot_decrypt_is_refused_with_its_own_reason() {
    // **「秘密鍵を指してください」は的外れ**です（指しているので）。
    // 使えない理由を分けないと、人は正しい鍵を疑いはじめます。
    //
    // 実測（2026-08-30・tests/key_formats_really_load.rs）:
    // - 暗号化された PKCS#8 は `russh` が復号できない
    // - PKCS#1 の AES-256-CBC は `russh` が `unimplemented!()` に落ちる（**アプリが落ちる**）
    let pkcs8 = inspect_key(b"-----BEGIN ENCRYPTED PRIVATE KEY-----\n");
    assert_eq!(pkcs8.format, KeyFormat::Pkcs8);
    assert_eq!(pkcs8.verdict, KeyVerdict::UnsupportedEncryption);
    assert!(
        !pkcs8.needs_passphrase,
        "入れても通らないパスフレーズを聞いている"
    );

    let aes256 = inspect_key(
        b"-----BEGIN RSA PRIVATE KEY-----\n\
          Proc-Type: 4,ENCRYPTED\n\
          DEK-Info: AES-256-CBC,E238345BDFC158AF353A7F7D72BC10B4\n",
    );
    assert_eq!(aes256.verdict, KeyVerdict::UnsupportedEncryption);
}

#[test]
fn a_public_key_is_named_rather_than_silently_accepted() {
    // **一番ありがちな取り違え。**`.pub` を指しても「鍵が違う」としか出ないと、
    // 人は何度でも同じ間違いをします。
    let facts = inspect_key(b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI... someone@example\n");
    assert_eq!(facts.format, KeyFormat::PublicKey);
    assert!(!facts.usable(), "公開鍵で認証しに行こうとしている");
}

#[test]
fn something_that_is_not_a_key_at_all_is_refused_rather_than_guessed() {
    let facts = inspect_key(b"# just a config file\nHost example\n");
    assert_eq!(facts.format, KeyFormat::Unknown);
    assert!(!facts.usable());
    assert!(!facts.needs_passphrase);
}

#[test]
fn a_leading_blank_line_or_a_bom_does_not_hide_the_format() {
    // Windows を経由した鍵は BOM や CRLF が付くことがあります（PRD §7）。
    let with_bom = format!("\u{feff}\r\n{}", ppk_header(3, "aes256-cbc"));
    assert_eq!(inspect_key(with_bom.as_bytes()).format, KeyFormat::Ppk3);
}

#[test]
fn a_binary_file_does_not_panic_the_inspection() {
    // 人はどんなファイルでも選べます。**落ちるより断る。**
    let facts = inspect_key(&[0x00, 0xff, 0xfe, 0x80, 0x01, 0x02]);
    assert_eq!(facts.format, KeyFormat::Unknown);
    assert!(!facts.usable());
}
