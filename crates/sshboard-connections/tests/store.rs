//! 接続一覧の読み書き。
//!
//! **一番大事なのは「AI へホスト名が漏れないこと」**（CLAUDE.md 禁止事項 5）。

use sshboard_connections::{Connections, ConnectionsError};

const SAMPLE: &str = r#"
version = 1

[[connections]]
id = "web-prod"
name = "Web (prod)"
host = "example.invalid"
port = 2222
user = "deploy"
key_path = "~/.ssh/id_ed25519"
keyring_passphrase_ref = "sshboard.web-prod.passphrase"
fingerprint = "SHA256:aaaa"

[[connections]]
id = "mail"
name = "Mail"
host = "mail.example.invalid"
user = "ops"
"#;

#[test]
fn a_registered_connection_is_read_back_with_every_field() {
    // Arrange & Act
    let connections = Connections::parse(SAMPLE).expect("読めない");

    // Assert
    assert_eq!(connections.connections.len(), 2);
    let entry = connections.get("web-prod").expect("web-prod が無い");
    assert_eq!(entry.port, 2222);
    assert_eq!(entry.user, "deploy");
    assert_eq!(
        entry.keyring_passphrase_ref.as_deref(),
        Some("sshboard.web-prod.passphrase")
    );
}

#[test]
fn the_port_defaults_to_twenty_two_when_it_is_left_out() {
    let connections = Connections::parse(SAMPLE).expect("読めない");

    assert_eq!(connections.get("mail").expect("mail が無い").port, 22);
}

#[test]
fn what_the_ai_sees_carries_no_host_and_no_user() {
    // **これが一番大事なテストです**（CLAUDE.md 禁止事項 5）。
    // Arrange
    let connections = Connections::parse(SAMPLE).expect("読めない");

    // Act
    let summaries = connections.summaries();
    let rendered = format!("{summaries:?}");

    // Assert
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, "web-prod");
    assert_eq!(summaries[0].name, "Web (prod)");
    assert!(
        !rendered.contains("example.invalid"),
        "ホスト名が漏れている: {rendered}"
    );
    assert!(
        !rendered.contains("deploy"),
        "利用者名が漏れている: {rendered}"
    );
    assert!(
        !rendered.contains("id_ed25519"),
        "鍵のパスが漏れている: {rendered}"
    );
    assert!(!rendered.contains("2222"), "ポートが漏れている: {rendered}");
}

#[test]
fn the_json_the_ai_receives_carries_no_host_either() {
    // Debug だけでなく、実際に送る形でも確かめる。
    // Arrange
    let connections = Connections::parse(SAMPLE).expect("読めない");

    // Act
    let json = serde_json::to_string(&connections.summaries()).expect("serialize できない");

    // Assert
    assert!(
        !json.contains("example.invalid"),
        "ホスト名が漏れている: {json}"
    );
    assert!(!json.contains("deploy"), "利用者名が漏れている: {json}");
}

#[test]
fn a_secret_is_never_stored_in_the_file_itself() {
    // 参照名だけを持つ（D11）。値は OS ストアにある。
    let connections = Connections::parse(SAMPLE).expect("読めない");
    let written = connections.to_toml().expect("書けない");

    assert!(
        written.contains("keyring_passphrase_ref"),
        "参照が消えている"
    );
    // 参照名は「どこに置いたか」であって、パスフレーズそのものではない
    assert!(
        !written.contains("passphrase = "),
        "パスフレーズが直接書かれている: {written}"
    );
}

#[test]
fn two_connections_with_the_same_id_are_rejected_instead_of_one_being_dropped() {
    // 黙って落とすと、人は「登録したはずなのに無い」に遭う。
    // Arrange
    let input = r#"
version = 1
[[connections]]
id = "same"
name = "One"
host = "a.invalid"
user = "u"
[[connections]]
id = "same"
name = "Two"
host = "b.invalid"
user = "u"
"#;

    // Act & Assert
    assert_eq!(
        Connections::parse(input),
        Err(ConnectionsError::DuplicateId {
            id: "same".to_owned()
        })
    );
}

#[test]
fn an_unknown_version_stops_instead_of_being_read_as_if_it_were_current() {
    // Arrange
    let input = "version = 99\n";

    // Act & Assert
    assert_eq!(
        Connections::parse(input),
        Err(ConnectionsError::UnknownVersion { found: 99 })
    );
}

#[test]
fn an_empty_id_is_rejected() {
    let input = r#"
version = 1
[[connections]]
id = ""
name = "No id"
host = "a.invalid"
user = "u"
"#;

    assert_eq!(Connections::parse(input), Err(ConnectionsError::EmptyId));
}

#[test]
fn a_missing_file_is_not_an_error_and_no_file_is_created() {
    // まだ 1 件も登録していないだけ。**読むだけでファイルを作らない。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("connections.toml");

    // Act
    let connections = Connections::load_or_empty(&path).expect("読めない");

    // Assert
    assert!(connections.connections.is_empty());
    assert!(!path.exists(), "読むだけでファイルを作っている");
}

#[test]
fn saving_then_loading_gives_back_the_same_thing() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("nested").join("connections.toml");
    let original = Connections::parse(SAMPLE).expect("読めない");

    // Act
    original.save(&path).expect("書けない");
    let read_back = Connections::load_or_empty(&path).expect("読めない");

    // Assert
    assert_eq!(read_back, original);
}

#[cfg(unix)]
#[test]
fn the_saved_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("connections.toml");

    // Act
    Connections::parse(SAMPLE)
        .expect("読めない")
        .save(&path)
        .expect("書けない");

    // Assert
    let mode = std::fs::metadata(&path)
        .expect("読めない")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "他の利用者に読めます: {mode:o}");
}
