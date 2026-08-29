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
    // **素の行を数に入れない。**このテストは以前 2 を期待していたが、
    // それは「22 番で確かめた鍵を 2222 番の相手へ流用する」という誤りだった。
    assert_eq!(other_port.len(), 1, "[host]:port の行だけを拾えていない");
}

#[test]
fn a_hashed_line_for_a_different_host_is_not_treated_as_a_match() {
    // ハッシュ化された行は読めるようになったが、**中身が別のホストなら一致させない。**
    // 通す方向へ倒さない。
    let file = "|1|abcdef=|ghijkl= ssh-ed25519 AAAA\n";

    assert!(fingerprints_for(file, "example.invalid", 22).is_empty());
}

#[test]
fn comments_and_blank_lines_are_skipped() {
    let file = "# comment\n\nexample.invalid ssh-ed25519 AAAA\n";

    assert_eq!(fingerprints_for(file, "example.invalid", 22).len(), 1);
}

#[test]
fn a_hashed_known_hosts_line_matches_the_host_it_was_made_for() {
    // **OpenSSH の既定はハッシュ化**（HashKnownHosts yes）。ここを読めないと、
    // 人が既に ssh で確かめて済ませた判断を全部捨てることになる。
    //
    // 下の 2 行は `ssh-keygen -H` が**実際に作ったもの**（合成のホスト名で）。
    // 1 行目が既定ポート、2 行目が `[host]:2222` の形。
    // Arrange
    let known = "\
|1|dss8sWTyfjVLUWSwDC27osdTkO4=|iCXCcJZjN4MrdLrG4PphBh99V38= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0
|1|gJvBxHoUPu4jsPdyiFETBQINUuk=|oxQw4DhNPCqWwgERIvtCOVuuO48= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ1
";

    // Act
    let default_port = fingerprints_for(known, "host.example.invalid", 22);
    let other_port = fingerprints_for(known, "host.example.invalid", 2222);
    let other_host = fingerprints_for(known, "elsewhere.example.invalid", 22);

    // Assert
    assert_eq!(default_port.len(), 1, "ハッシュ化された行を読めていない");
    assert!(default_port[0].starts_with("SHA256:"));
    // **ポートが違えば別の記録。**`[host]:2222` の形もハッシュ化されている。
    assert_eq!(other_port.len(), 1, "ポート付きの行を読めていない");
    assert_ne!(
        default_port[0], other_port[0],
        "ポートで引き分けられていない"
    );
    assert!(other_host.is_empty(), "別のホストに一致してしまっている");
}

#[test]
fn a_malformed_hashed_line_is_skipped_rather_than_crashing() {
    // 壊れた行 1 つで known_hosts 全体が読めなくなると、**繋げない理由が分からなくなる。**
    let broken = "|1|not-base64|also-not-base64 ssh-ed25519 AAAA\n\
                  |1|onlyonefield ssh-ed25519 AAAA\n\
                  |9|F1S6yTfoBAOA9m5nJU4/e6r1xIY=|nGpDcbBK+bUeS4Ho00LOTVYuiZg= ssh-ed25519 AAAA\n";

    assert!(fingerprints_for(broken, "host.example.invalid", 22).is_empty());
}

#[test]
fn a_key_recorded_for_the_default_port_is_not_reused_for_another_port() {
    // **ポート 22 で確かめた鍵を、別ポートの相手に流用しない。**
    // 同じホスト名でも、別ポートで待っているのは別のサービスでありうる。
    // OpenSSH も `[host]:port` の形でしか記録しない。
    // Arrange
    let known = "host.example.invalid ssh-ed25519 \
                 AAAAC3NzaC1lZDI1NTE5AAAAIJZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0nQZ0\n";

    // Act & Assert
    assert_eq!(fingerprints_for(known, "host.example.invalid", 22).len(), 1);
    assert!(
        fingerprints_for(known, "host.example.invalid", 2222).is_empty(),
        "既定ポートの記録を別ポートへ流用している"
    );
}
