//! ホスト鍵の判断。**「とりあえず通す」が作れないことを見張る。**

use sshboard_ssh::{decide, fingerprint, fingerprints_for, SeenHostKey, Trust};

fn seen(fp: &str) -> SeenHostKey {
    SeenHostKey {
        algorithm: "ssh-ed25519".into(),
        fingerprint: fp.into(),
    }
}

#[test]
fn the_fingerprint_matches_the_form_ssh_keygen_prints() {
    assert_eq!(
        fingerprint(b""),
        "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU"
    );
}

#[test]
fn a_first_time_host_is_not_acceptable_on_its_own() {
    // **初めて見るホストを黙って通さない。**人が確かめる必要がある。
    let trust = decide(&seen("SHA256:aaa"), None, &[]);

    assert_eq!(trust, Trust::Unknown);
    assert!(!trust.is_acceptable(), "初見のホストを通している");
}

#[test]
fn a_key_that_disagrees_with_the_pin_is_refused() {
    // すり替えの可能性。**通す方向へ倒さない。**
    let trust = decide(
        &seen("SHA256:bbb"),
        Some("SHA256:aaa"),
        &["SHA256:bbb".into()],
    );

    assert_eq!(
        trust,
        Trust::Mismatch {
            expected: "SHA256:aaa".into()
        }
    );
    assert!(!trust.is_acceptable(), "食い違う鍵を通している");
}

#[test]
fn a_pinned_key_wins_over_known_hosts() {
    // 登録した指紋がある接続では、known_hosts より登録を優先する。
    let trust = decide(&seen("SHA256:aaa"), Some("SHA256:aaa"), &[]);

    assert_eq!(trust, Trust::Pinned);
    assert!(trust.is_acceptable());
}

#[test]
fn a_host_listed_in_known_hosts_is_acceptable() {
    let trust = decide(&seen("SHA256:aaa"), None, &["SHA256:aaa".into()]);

    assert_eq!(trust, Trust::KnownHosts);
    assert!(trust.is_acceptable());
}

#[test]
fn known_hosts_entries_are_matched_by_host_and_port() {
    // Arrange — 22 番以外は [host]:port の形で書かれる
    let file = "\
example.invalid ssh-ed25519 AAAA
[example.invalid]:2222 ssh-ed25519 AAAB
other.invalid ssh-ed25519 AAAC
";

    // Act
    let default_port = fingerprints_for(file, "example.invalid", 22);
    let other_port = fingerprints_for(file, "example.invalid", 2222);

    // Assert
    assert_eq!(default_port.len(), 1, "22 番の行を拾えていない");
    assert_eq!(other_port.len(), 2, "[host]:port の行を拾えていない");
}

#[test]
fn a_hashed_known_hosts_line_is_not_treated_as_a_match() {
    // **読めない行を「載っていない」と扱う。**通す方向へ倒さない。
    let file = "|1|abcdef=|ghijkl= ssh-ed25519 AAAA\n";

    assert!(fingerprints_for(file, "example.invalid", 22).is_empty());
}

#[test]
fn comments_and_blank_lines_are_skipped() {
    let file = "# comment\n\nexample.invalid ssh-ed25519 AAAA\n";

    assert_eq!(fingerprints_for(file, "example.invalid", 22).len(), 1);
}
