//! Issue 001 の完了条件を、ヘッドレスで押さえるテスト。
//!
//! HTTP を経由しない。ここで見るのは「ツール応答より先に帯へ出るか」だけ。

use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_mcp::SshboardMcp;
use sshboard_stream::OutputStream;

#[tokio::test]
async fn ping_answers_pong() {
    // Arrange
    let server = SshboardMcp::new(Band::new(), Arc::new(OutputStream::new()));

    // Act
    let answer = server.ping().await.expect("ping が失敗した");

    // Assert
    assert_eq!(answer, "pong");
}

#[tokio::test]
async fn ping_puts_an_ai_line_on_the_band() {
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    tokio::spawn(async move { server.ping().await });
    let event = subscriber.recv().await.expect("帯へ出ていない");

    // Assert
    assert_eq!(event.line().actor(), Actor::Ai);
    assert_eq!(event.line().text(), "ping");
    assert!(
        event.line().render().starts_with("[AI]"),
        "実際: {:?}",
        event.line().render()
    );
}

#[tokio::test]
async fn ping_does_not_answer_until_the_band_has_the_line() {
    // 001 の本命。AI が返答したあとに画面が追いつく形になっていないこと。
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    let mut call = tokio::spawn(async move { server.ping().await });
    let answered_before_the_band = tokio::time::timeout(Duration::from_millis(100), &mut call)
        .await
        .is_ok();

    subscriber.recv().await.expect("帯へ出ていない").ack();
    let answer = tokio::time::timeout(Duration::from_secs(2), call)
        .await
        .expect("ack したのに応答が返らない")
        .expect("ツールがパニックした")
        .expect("ping が失敗した");

    // Assert
    assert!(
        !answered_before_the_band,
        "帯が受け取る前に応答が返っている"
    );
    assert_eq!(answer, "pong");
}

#[tokio::test]
async fn ping_fails_when_the_screen_never_confirms() {
    // 画面が固まっているのに応答だけ返すと、見えないまま操作が進む。
    // Arrange
    let band = Band::new();
    let _stuck_screen = band.subscribe(); // 受け取るが ack しない
    let server = SshboardMcp::with_ack_timeout(
        band,
        Arc::new(OutputStream::new()),
        Duration::from_millis(50),
    );

    // Act
    let result = server.ping().await;

    // Assert
    assert!(
        result.is_err(),
        "帯が受け取っていないのに成功している: {result:?}"
    );
}

#[tokio::test]
async fn read_stream_returns_the_plain_tail_without_any_ansi() {
    // GUI には色が残り、MCP には残らない（Issue 005）。
    // Arrange
    let stream = Arc::new(OutputStream::new());
    stream
        .push(b"\x1b[31mdisk full\x1b[0m\r\n")
        .expect("流せない");
    let server = SshboardMcp::new(Band::new(), Arc::clone(&stream));

    // Act
    let text = server.read_stream().await.expect("読めない");

    // Assert
    assert_eq!(text, "disk full\n");
    assert!(!text.contains('\x1b'), "ANSI が混ざっている: {text:?}");
}

#[tokio::test]
async fn read_stream_puts_an_ai_line_on_the_band_before_answering() {
    // 読んだことも帯に出る（PRD §4-2）。
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()));

    // Act
    let mut call = tokio::spawn(async move { server.read_stream().await });
    let answered_before_the_band = tokio::time::timeout(Duration::from_millis(100), &mut call)
        .await
        .is_ok();
    let event = subscriber.recv().await.expect("帯へ出ていない");
    let line = event.line().clone();
    event.ack();
    call.await.expect("パニック").expect("読めない");

    // Assert
    assert!(
        !answered_before_the_band,
        "帯が受け取る前に応答が返っている"
    );
    assert_eq!(line.actor(), Actor::Ai);
    assert_eq!(line.text(), "read_stream");
}

/// 接続一覧の中身。**ホスト名も利用者名も入れてある。**
/// AI 側へ漏れないことを見張るために、あえて入れている。
const CONNECTIONS: &str = r#"
version = 1

[[connections]]
id = "web-prod"
name = "Web (prod)"
host = "secret-host.invalid"
port = 2222
user = "secret-user"
"#;

fn connections_file() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, CONNECTIONS).expect("書けない");
    (dir, path)
}

#[tokio::test]
async fn list_connections_returns_identifiers_and_nothing_else() {
    // **これが D11 と CLAUDE.md 禁止事項 5 の見張りです。**
    // Arrange
    let (_dir, path) = connections_file();
    let server =
        SshboardMcp::new(Band::new(), Arc::new(OutputStream::new())).with_connections(path);

    // Act
    let rendered = server.list_connections().await.expect("読めない");
    let listed: serde_json::Value = serde_json::from_str(&rendered).expect("JSON でない");

    // Assert
    assert_eq!(listed.as_array().expect("配列でない").len(), 1);
    assert_eq!(listed[0]["id"], "web-prod");
    assert_eq!(listed[0]["name"], "Web (prod)");
    assert!(
        !rendered.contains("secret-host"),
        "ホスト名が漏れている: {rendered}"
    );
    assert!(
        !rendered.contains("secret-user"),
        "利用者名が漏れている: {rendered}"
    );
    assert!(!rendered.contains("2222"), "ポートが漏れている: {rendered}");
}

#[tokio::test]
async fn list_connections_with_nothing_registered_is_empty_not_an_error() {
    // まだ 1 件も登録していないだけ。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let server = SshboardMcp::new(Band::new(), Arc::new(OutputStream::new()))
        .with_connections(dir.path().join("connections.toml"));

    // Act
    let rendered = server.list_connections().await.expect("読めない");

    // Assert
    assert_eq!(rendered, "[]");
}

#[tokio::test]
async fn list_connections_puts_an_ai_line_on_the_band() {
    // 接続一覧を見たことも帯に出る（PRD §4-2）。
    // Arrange
    let (_dir, path) = connections_file();
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new())).with_connections(path);

    // Act
    tokio::spawn(async move { server.list_connections().await });
    let event = subscriber.recv().await.expect("帯へ出ていない");

    // Assert
    assert_eq!(event.line().actor(), Actor::Ai);
    assert_eq!(event.line().text(), "list_connections");
}

use rmcp::handler::server::wrapper::Parameters;
use sshboard_mcp::RegisterConnection;

fn registration(id: &str) -> Parameters<RegisterConnection> {
    Parameters(RegisterConnection {
        id: id.to_owned(),
        name: format!("{id} の名前"),
        host: "secret-host.invalid".to_owned(),
        port: 2222,
        user: "secret-user".to_owned(),
        key_path: None,
    })
}

#[tokio::test]
async fn register_connection_writes_to_the_local_file() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("connections.toml");
    let server =
        SshboardMcp::new(Band::new(), Arc::new(OutputStream::new())).with_connections(path.clone());

    // Act
    server
        .register_connection(registration("web-prod"))
        .await
        .expect("登録できない");

    // Assert
    let written = std::fs::read_to_string(&path).expect("ファイルが無い");
    assert!(written.contains("web-prod"), "書かれていない: {written}");
    assert!(
        written.contains("2222"),
        "ポートが書かれていない: {written}"
    );
}

#[tokio::test]
async fn register_connection_tells_the_band_the_id_but_not_the_host() {
    // **帯は画面に出る。**接続先を帯へ出すと、画面の写真に写る（PRD §8）。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let server = SshboardMcp::new(band, Arc::new(OutputStream::new()))
        .with_connections(dir.path().join("connections.toml"));

    // Act
    tokio::spawn(async move { server.register_connection(registration("web-prod")).await });
    let event = subscriber.recv().await.expect("帯へ出ていない");
    let rendered = event.line().render();
    event.ack();

    // Assert
    assert_eq!(event.line().actor(), Actor::Ai);
    assert!(
        rendered.contains("web-prod"),
        "識別子が出ていない: {rendered}"
    );
    assert!(
        !rendered.contains("secret-host"),
        "ホスト名が帯に出ている: {rendered}"
    );
    assert!(
        !rendered.contains("secret-user"),
        "利用者名が帯に出ている: {rendered}"
    );
}

#[tokio::test]
async fn register_connection_refuses_a_duplicate_id_instead_of_overwriting() {
    // 黙って上書きすると、人が登録したものが消える。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let path = dir.path().join("connections.toml");
    let server =
        SshboardMcp::new(Band::new(), Arc::new(OutputStream::new())).with_connections(path);

    // Act
    server
        .register_connection(registration("same"))
        .await
        .expect("1 回目が失敗した");
    let second = server.register_connection(registration("same")).await;

    // Assert
    assert!(second.is_err(), "重複を黙って受け入れている");
}

#[tokio::test]
async fn register_connection_rejects_an_unusable_id() {
    // 識別子はファイルにも帯にも出る。**変な文字を通さない。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れない");
    let server = SshboardMcp::new(Band::new(), Arc::new(OutputStream::new()))
        .with_connections(dir.path().join("connections.toml"));

    // Act
    let mut bad = registration("web prod");
    bad.0.id = "web prod".to_owned();
    let result = server.register_connection(bad).await;

    // Assert
    assert!(result.is_err(), "空白入りの識別子を通している");
}
