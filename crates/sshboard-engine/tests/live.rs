//! 実行体を実機で。**サーバーが無い環境でも走ります**（product-baseline §4）。
//!
//! 建てるには `sh tools/test-server/up.sh`。

use std::path::PathBuf;
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_ssh::{Auth, SshError, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 2222;
const USER: &str = "probe";

async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

/// テスト用サーバーの指紋。**1 回だけ調べて使い回す**（sshd の MaxStartups 対策）。
static FINGERPRINT: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

async fn known_fingerprint() -> &'static String {
    FINGERPRINT
        .get_or_init(|| async {
            let target = Target {
                host: HOST.into(),
                port: PORT,
                user: USER.into(),
                pinned_fingerprint: None,
                known_hosts: String::new(),
                write_scope: WriteScope::default(),
            };
            match SshSession::connect(&target, &Auth::Agent, Band::new()).await {
                Err(SshError::UntrustedHost { seen, .. }) => seen.fingerprint,
                Ok(_) => panic!("初見のホストを通している"),
                Err(other) => panic!("繋げません: {other}"),
            }
        })
        .await
}

/// 接続一覧を 1 件だけ書いた一時ファイルを作る。
async fn registry(dir: &tempfile::TempDir, write_roots: &[&str]) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let roots = write_roots
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let toml = format!(
        "version = 1\n\n[[connections]]\nid = \"local\"\nname = \"Local test server\"\n\
         host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\n\
         fingerprint = \"{}\"\ntag = \"test\"\nwrite_roots = [{roots}]\n",
        known_fingerprint().await
    );
    std::fs::write(&path, toml).expect("接続一覧を書けない");
    path
}

fn engine_at(path: PathBuf) -> Engine {
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn nothing_can_be_done_before_a_connection_is_open() {
    // **繋がっていないのに動く経路があってはいけない**（裏で張ってしまうから）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    assert!(engine.current().await.is_none());
    assert!(matches!(
        engine.list_dir(Actor::Ai, "/").await,
        Err(EngineError::NotConnected)
    ));
    assert!(matches!(
        engine.upload_bytes(Actor::Human, "/tmp/x", b"x").await,
        Err(EngineError::NotConnected)
    ));
}

#[tokio::test]
async fn an_unknown_connection_is_named_in_the_refusal() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    let result = engine.connect(Actor::Ai, "does-not-exist", None).await;

    match result {
        Err(EngineError::UnknownConnection(id)) => assert_eq!(id, "does-not-exist"),
        other => panic!("識別子を返していない: {:?}", other.map(|o| o.id)),
    }
}

#[tokio::test]
async fn opening_a_connection_publishes_it_and_carries_the_write_scope() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &["/home/probe/upload"]).await);
    let mut watching = engine.subscribe();

    let opened = engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    assert_eq!(opened.id, "local");
    assert_eq!(
        opened.write.ai_roots,
        vec!["/home/probe/upload".to_string()]
    );
    assert!(opened.write.human_unrestricted);
    assert!(opened.fingerprint.starts_with("SHA256:"));

    // **画面が知らないまま繋がっている、を作らない。**
    watching.changed().await.expect("配られていない");
    assert_eq!(
        watching.borrow().as_ref().map(|o| o.id.clone()),
        Some("local".to_string())
    );

    assert_eq!(engine.current().await.map(|o| o.id), Some("local".into()));
}

#[tokio::test]
async fn what_the_engine_publishes_carries_no_host_and_no_user() {
    // **AI へホストと利用者名を渡さない**（CLAUDE.md 禁止事項 5）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &[]).await);
    let opened = engine
        .connect(Actor::Ai, "local", None)
        .await
        .expect("繋がらない");

    let rendered = format!("{opened:?}");
    assert!(!rendered.contains(USER), "利用者名が漏れている: {rendered}");
    assert!(!rendered.contains(HOST), "ホストが漏れている: {rendered}");
}

#[tokio::test]
async fn a_second_connect_is_refused_rather_than_opening_a_hidden_session() {
    // **裏で見えない SSH を 1 本も増やさない**（PRD §4-1）。ここが崩れたら製品の意味が消える。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &[]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    let again = engine.connect(Actor::Ai, "local", None).await;

    assert!(
        matches!(again, Err(EngineError::AlreadyConnected { .. })),
        "2 本目を開いている"
    );
}

#[tokio::test]
async fn disconnecting_clears_the_current_connection_and_is_safe_to_repeat() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &[]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    assert_eq!(
        engine.disconnect(Actor::Human).await.map(|o| o.id),
        Some("local".to_string())
    );
    assert!(engine.current().await.is_none());
    // 2 回目は「何も開いていなかった」。**失敗にしない。**
    assert!(engine.disconnect(Actor::Human).await.is_none());
}

#[tokio::test]
async fn an_ai_upload_is_bounded_by_the_write_roots_of_that_connection() {
    // **D22 を、接続一覧の設定から実機まで通しで確かめる。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &["/home/probe/upload"]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    engine
        .ensure_dir(Actor::Ai, "/home/probe/upload")
        .await
        .expect("囲いの中なのに作れない");
    engine
        .upload_bytes(Actor::Ai, "/home/probe/upload/via-engine.txt", b"engine")
        .await
        .expect("囲いの中なのに上げられない");

    let refused = engine
        .upload_bytes(Actor::Ai, "/home/probe/outside.txt", b"nope")
        .await;
    assert!(
        matches!(refused, Err(EngineError::Ssh(SshError::WriteRefused(_)))),
        "囲いの外へ AI が書けている: {refused:?}"
    );

    let back = engine
        .read_file(Actor::Human, "/home/probe/upload/via-engine.txt")
        .await
        .expect("読み戻せない");
    assert_eq!(back, b"engine");
}

#[tokio::test]
async fn a_local_file_is_uploaded_byte_for_byte() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &["/home/probe/upload"]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    // テキストとして触ると壊れる中身で確かめる。
    let payload: Vec<u8> = b"\x00\xff\r\n\xe3\x81\x82".to_vec();
    let local = dir.path().join("artifact.bin");
    std::fs::write(&local, &payload).expect("手元に書けない");

    engine
        .ensure_dir(Actor::Human, "/home/probe/upload/release")
        .await
        .expect("作れない");
    let written = engine
        .upload_file(
            Actor::Human,
            &local,
            "/home/probe/upload/release/artifact.bin",
        )
        .await
        .expect("上げられない");

    assert_eq!(written, payload.len() as u64);
    let back = engine
        .read_file(Actor::Human, "/home/probe/upload/release/artifact.bin")
        .await
        .expect("読み戻せない");
    assert_eq!(back, payload, "上げたものと落としたものが違う");
}

#[tokio::test]
async fn a_missing_local_file_is_reported_as_a_local_problem_not_an_ssh_one() {
    // どちら側で失敗したのかが分からないと、人は直せない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry(&dir, &["/home/probe/upload"]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    let result = engine
        .upload_file(
            Actor::Human,
            &dir.path().join("no-such-file"),
            "/home/probe/upload/x",
        )
        .await;

    assert!(matches!(result, Err(EngineError::Local(_))), "{result:?}");
}
