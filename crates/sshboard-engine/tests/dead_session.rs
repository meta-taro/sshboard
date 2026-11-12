//! **下の SSH が切れた接続を、表に残さない**（2026-11-12・実機）。
//!
//! doko005 への SSH が切れていたのに、表には「開いている」と残った。
//! 端末を開こうとすると `Channel send error`、繋ぎ直そうとすると
//! 「すでに繋がっています。先に切ってから繋ぎ直してください」と断られ、
//! **画面から抜ける道がありませんでした**（切る釦が見つからない作りでもあった）。
//!
//! 切れた SSH は、テスト用サーバーの中でその利用者の sshd を落として作ります
//! （ネットワークが落ちた・相手が切った、と同じ形）。
//!
//! **サーバーが無い・docker が使えない環境では飛ばします**（product-baseline §4）。

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use sshboard_band::{Actor, Band};
use sshboard_diag::Diagnostics;
use sshboard_engine::Engine;
use sshboard_ssh::{Auth, SshError, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 2222;
const USER: &str = "probe";
const CONTAINER: &str = "sshboard-test-server";

/// 切れたことに実行体が気づくまで待つ長さ。
const NOTICE_WITHIN: Duration = Duration::from_secs(10);

async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

/// 利用者 probe の sshd を全部落とす。**docker が使えなければ `false`。**
fn drop_probe_sessions() -> bool {
    let script = "for p in /proc/[0-9]*; do \
                    if tr '\\0' ' ' < $p/cmdline 2>/dev/null | grep -Eq '^sshd(-session)?: probe'; then \
                      kill ${p#/proc/}; fi; done";
    Command::new("docker")
        .args(["exec", CONTAINER, "sh", "-c", script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn fingerprint() -> String {
    let target = Target {
        id: Some("local".into()),
        host: HOST.into(),
        port: PORT,
        user: USER.into(),
        pinned_fingerprint: None,
        known_hosts: String::new(),
        write_scope: WriteScope::default(),
    };
    match SshSession::connect(&target, &Auth::Agent, Band::new(), &Diagnostics::new()).await {
        Err(SshError::UntrustedHost { seen, .. }) => seen.fingerprint,
        Ok(_) => panic!("初見のホストを通している"),
        Err(other) => panic!("繋げません: {other}"),
    }
}

async fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path: PathBuf = dir.path().join("connections.toml");
    std::fs::write(
        &path,
        format!(
            "version = 1\n\n[[connections]]\nid = \"local\"\nname = \"Local test server\"\n\
             host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\nfingerprint = \"{}\"\n",
            fingerprint().await
        ),
    )
    .expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

/// 切れたことが表に出るまで待つ。**出なければ `false`。**
async fn noticed_gone(engine: &Engine) -> bool {
    let deadline = tokio::time::Instant::now() + NOTICE_WITHIN;
    while tokio::time::Instant::now() < deadline {
        if engine.open_connections().await.is_empty() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

#[tokio::test]
async fn a_connection_whose_ssh_dropped_is_not_shown_as_open_and_can_be_opened_again() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir).await;
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("最初の接続が通らない");
    assert_eq!(engine.open_connections().await.len(), 1);

    // Act —— 相手の側で SSH を落とす（ネットワークが落ちたのと同じ形）。
    if !drop_probe_sessions() {
        println!("docker で sshd を落とせません（想定内・飛ばします）");
        return;
    }

    // Assert —— 表から消える。**「開いている」と嘘を言わない。**
    assert!(
        noticed_gone(&engine).await,
        "SSH が切れたのに、{NOTICE_WITHIN:?} たっても表に「開いている」と残っている"
    );
    // **そのまま繋ぎ直せる。**「すでに繋がっています」で行き止まりにしない。
    let again = engine.connect(Actor::Human, "local", None).await;
    assert!(again.is_ok(), "切れた接続へ繋ぎ直せない: {again:?}");
}
