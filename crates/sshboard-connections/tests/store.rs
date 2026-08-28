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

#[tokio::test]
async fn a_change_notice_reaches_whoever_is_watching() {
    // 一覧は人（GUI）と AI（MCP）の両方が書き換える。
    // **押し出さないと、AI が足した接続を人が知らないままになる**（PRD §4-2）。
    // Arrange
    let watch = sshboard_connections::ConnectionsWatch::new();
    let mut watcher = watch.subscribe();

    // Act
    watch.notify();

    // Assert
    assert!(watcher.recv().await.is_ok(), "変更が届いていない");
}

#[tokio::test]
async fn notifying_with_nobody_watching_is_not_an_error() {
    // 画面をまだ開いていないだけ。
    let watch = sshboard_connections::ConnectionsWatch::new();

    watch.notify(); // panic しなければよい
}

#[tokio::test]
async fn the_notice_carries_no_connection_details() {
    // 中身を流すと、購読者ごとに接続先が配られる（PRD §8）。
    // 型が `()` である時点で運べないが、**変えたら気づけるように**置いておく。
    let watch = sshboard_connections::ConnectionsWatch::new();
    let mut watcher = watch.subscribe();
    watch.notify();

    let received: () = watcher.recv().await.expect("届いていない");
    assert_eq!(received, ());
}

const MARKED: &str = r#"
version = 1

[[connections]]
id = "prod"
name = "本番"
host = "secret-host.invalid"
user = "secret-user"
color = "red"
tag = "本番"
"#;

#[test]
fn a_mark_is_read_back() {
    // Arrange & Act
    let connections = Connections::parse(MARKED).expect("読めない");

    // Assert
    let entry = connections.get("prod").expect("prod が無い");
    assert_eq!(entry.color.as_deref(), Some("red"));
    assert_eq!(entry.tag.as_deref(), Some("本番"));
}

#[test]
fn the_tag_reaches_the_ai_but_the_host_still_does_not() {
    // **本番と開発の区別が付くこと自体が安全側に効く。**
    // ただしホスト名と利用者名は、これまでどおり渡さない。
    // Arrange
    let connections = Connections::parse(MARKED).expect("読めない");

    // Act
    let json = serde_json::to_string(&connections.summaries()).expect("serialize できない");

    // Assert
    assert!(json.contains("本番"), "タグが渡っていない: {json}");
    assert!(
        !json.contains("secret-host"),
        "ホスト名が漏れている: {json}"
    );
    assert!(
        !json.contains("secret-user"),
        "利用者名が漏れている: {json}"
    );
    assert!(!json.contains("red"), "色は AI に要らない: {json}");
}

#[test]
fn a_colour_outside_the_palette_is_rejected() {
    // 16 進数や勝手な名前を書くと、**対応する配色の定義が無い値がファイルに入る。**
    // Arrange
    let input = MARKED.replace(r#"color = "red""#, r##"color = "#1a73e8""##);

    // Act
    let result = Connections::parse(&input);

    // Assert
    assert!(
        matches!(result, Err(ConnectionsError::UnknownColor { .. })),
        "実際: {result:?}"
    );
}

#[test]
fn a_tag_is_measured_in_characters_not_bytes() {
    // 漢字 12 文字は 36 バイト。**バイトで測ると短い方のラベルを弾く。**
    // Arrange
    let just_fits = "本番環境検証開発予備一二"; // 12 文字
    let one_too_many = "本番環境検証開発予備一二三"; // 13 文字
    assert_eq!(just_fits.chars().count(), 12, "テストの前提が崩れている");
    assert_eq!(one_too_many.chars().count(), 13, "テストの前提が崩れている");
    // 12 文字の漢字は 36 バイト。**バイトで測る実装なら、ここで弾かれる。**
    assert!(just_fits.len() > 12, "テストの前提が崩れている");

    let ok = MARKED.replace(r#"tag = "本番""#, &format!(r#"tag = "{just_fits}""#));
    let too_long = MARKED.replace(r#"tag = "本番""#, &format!(r#"tag = "{one_too_many}""#));

    // Act & Assert
    assert!(Connections::parse(&ok).is_ok(), "12 文字を弾いている");
    assert!(
        matches!(
            Connections::parse(&too_long),
            Err(ConnectionsError::TagTooLong { .. })
        ),
        "13 文字を通している"
    );
}

#[test]
fn the_write_roots_survive_a_round_trip_and_default_to_empty() {
    // **既定が空でないと、設定を忘れた接続で AI が書けてしまう**（D22）。
    // Arrange
    let with_roots = r#"
version = 1

[[connections]]
id = "app"
name = "App"
host = "192.0.2.10"
user = "deploy"
write_roots = ["/srv/app/releases", "/srv/app/shared"]

[[connections]]
id = "plain"
name = "Plain"
host = "192.0.2.11"
user = "deploy"
"#;

    // Act
    let connections = Connections::parse(with_roots).expect("読めない");
    let again =
        Connections::parse(&connections.to_toml().expect("書けない")).expect("読み直せない");

    // Assert
    assert_eq!(
        again.get("app").expect("app が無い").write_roots,
        vec![
            "/srv/app/releases".to_string(),
            "/srv/app/shared".to_string()
        ]
    );
    assert!(
        again
            .get("plain")
            .expect("plain が無い")
            .write_roots
            .is_empty(),
        "書き込み許可を書いていない接続に許可が生えている"
    );
}

#[test]
fn the_ai_is_told_where_it_may_write_but_still_not_where_the_server_is() {
    // 許可ディレクトリを隠すと、AI は毎回断られて理由が分からない。
    // ただし**ホストと利用者名は依然として渡さない**（CLAUDE.md 禁止事項 5）。
    // Arrange
    let source = r#"
version = 1

[[connections]]
id = "app"
name = "App"
host = "secret.example.invalid"
user = "deployuser"
write_roots = ["/srv/app/releases"]
"#;
    let connections = Connections::parse(source).expect("読めない");

    // Act
    let json = serde_json::to_string(&connections.summaries()).expect("JSON にできない");

    // Assert
    assert!(
        json.contains("/srv/app/releases"),
        "許可先が伝わらない: {json}"
    );
    assert!(
        !json.contains("secret.example.invalid"),
        "ホストが漏れている: {json}"
    );
    assert!(!json.contains("deployuser"), "利用者名が漏れている: {json}");
}
