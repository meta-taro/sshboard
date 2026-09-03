//! 書き出したものが、同じパスフレーズでだけ戻ること（D18）。
//!
//! **この層が壊れると、接続先と鍵のパスフレーズが平文で外へ出ます。**
//! 通ることより、**通らないべきものが通らないこと**を厚く見ています。

use sshboard_bundle::{
    decrypt_bundle, encrypt_bundle, BundleError, BundlePayload, MIN_PASSPHRASE_LEN,
};
use sshboard_connections::{ConnectionEntry, Connections};

const PASS: &str = "sshboard-test-pass";

fn sample() -> BundlePayload {
    let entry = ConnectionEntry {
        id: "prod".into(),
        name: "本番".into(),
        host: "192.0.2.10".into(),
        port: 22,
        user: "deploy".into(),
        key_path: Some("/home/me/.ssh/id_ed25519".into()),
        keyring_passphrase_ref: Some("prod-key".into()),
        keyring_password_ref: None,
        fingerprint: Some("SHA256:xxxx".into()),
        known_hosts: None,
        color: Some("red".into()),
        tag: Some("本番".into()),
        write_roots: vec!["/srv/app".into()],
    };
    let mut secrets = std::collections::BTreeMap::new();
    secrets.insert("prod-key".to_string(), "鍵のパスフレーズ".to_string());
    BundlePayload::new(
        Connections {
            version: 1,
            connections: vec![entry],
        },
        secrets,
    )
}

#[test]
fn what_goes_in_comes_back_out() {
    let blob = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    let back = decrypt_bundle(&blob, PASS).expect("読み戻せない");
    assert_eq!(back.connections.connections.len(), 1);
    assert_eq!(back.connections.connections[0].host, "192.0.2.10");
    assert_eq!(
        back.secrets.get("prod-key").map(String::as_str),
        Some("鍵のパスフレーズ")
    );
}

#[test]
fn the_blob_does_not_carry_the_host_or_the_secret_in_the_clear() {
    // **ここが要。**暗号化したつもりで平文が混ざっていたら、
    // ファイルを渡した相手以外にも接続先が漏れます。
    let blob = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    let text = String::from_utf8_lossy(&blob);
    for forbidden in [
        "192.0.2.10",
        "deploy",
        "鍵のパスフレーズ",
        "id_ed25519",
        "本番",
    ] {
        assert!(!text.contains(forbidden), "平文で混ざっている: {forbidden}");
    }
}

#[test]
fn a_wrong_passphrase_is_told_apart_from_a_broken_file() {
    // **人の次の一手が違います。**打ち間違いなら打ち直す。壊れているなら貰い直す。
    let blob = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    assert!(matches!(
        decrypt_bundle(&blob, "まちがったパスフレーズ"),
        Err(BundleError::IncorrectPassphrase)
    ));
}

#[test]
fn a_file_that_is_not_a_bundle_is_refused_as_corrupt_not_as_a_wrong_passphrase() {
    let junk = b"this is not an age file at all";
    assert!(matches!(
        decrypt_bundle(junk, PASS),
        Err(BundleError::Corrupt)
    ));
}

#[test]
fn a_tampered_body_is_refused() {
    // **中身を 1 バイト書き換えたら開かない**こと。
    let mut blob = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    let last = blob.len() - 1;
    blob[last] ^= 0xFF;
    assert!(
        decrypt_bundle(&blob, PASS).is_err(),
        "改竄した中身が通っている"
    );
}

#[test]
fn a_short_passphrase_is_refused_before_anything_is_written() {
    // **空や 1 文字を「暗号化した」と言わない。**
    let short = "a".repeat(MIN_PASSPHRASE_LEN - 1);
    assert!(matches!(
        encrypt_bundle(&sample(), &short),
        Err(BundleError::WeakPassphrase)
    ));
    assert!(matches!(
        encrypt_bundle(&sample(), ""),
        Err(BundleError::WeakPassphrase)
    ));
}

#[test]
fn the_same_content_does_not_produce_the_same_file() {
    // 塩が効いていること。**同じに見えるファイルは、比較で中身を推測されます。**
    let a = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    let b = encrypt_bundle(&sample(), PASS).expect("書き出せない");
    assert_ne!(a, b, "2 回書き出して同じになっている");
}

#[test]
fn the_debug_output_never_shows_a_secret() {
    // **`{:?}` は事故の入口。**ログにもパニックにも出ます。
    let shown = format!("{:?}", sample());
    assert!(
        !shown.contains("鍵のパスフレーズ"),
        "Debug に秘密が出ている"
    );
}
