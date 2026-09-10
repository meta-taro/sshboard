//! 実行体を実機で。**サーバーが無い環境でも走ります**（product-baseline §4）。
//!
//! 建てるには `sh tools/test-server/up.sh`。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_diag::Diagnostics;
use sshboard_engine::{Engine, EngineError, OnConflict};
use sshboard_ssh::{Auth, SshError, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;

mod common;
use common::toml_string;

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
                id: Some("local".into()),
                host: HOST.into(),
                port: PORT,
                user: USER.into(),
                pinned_fingerprint: None,
                known_hosts: String::new(),
                write_scope: WriteScope::default(),
            };
            match SshSession::connect(&target, &Auth::Agent, Band::new(), &Diagnostics::new()).await
            {
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

/// 同じ相手を 2 つの識別子で登録する。**別々の接続として開けるか**を見るため。
async fn registry_pair(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let fingerprint = known_fingerprint().await;
    let one = |id: &str, name: &str| {
        format!(
            "[[connections]]\nid = \"{id}\"\nname = \"{name}\"\n\
             host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\n\
             fingerprint = \"{fingerprint}\"\n"
        )
    };
    std::fs::write(
        &path,
        format!(
            "version = 1\n\n{}\n{}",
            one("first", "One"),
            one("second", "Two")
        ),
    )
    .expect("接続一覧を書けない");
    path
}

fn engine_at(path: PathBuf) -> Engine {
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

/// AI に端末を開かせる。**D42 が入ったので、頼んで人が許すまで開けません。**
async fn ai_console_open(engine: &Engine) {
    let asked = engine.console_open(Actor::Ai, 80, 24).await;
    assert!(
        matches!(asked, Err(EngineError::ConsoleApprovalNeeded)),
        "AI が許可なく端末を開けてしまう: {asked:?}"
    );
    engine
        .console_answer(Actor::Human, true)
        .await
        .expect("人が許せない");
    engine
        .console_open(Actor::Ai, 80, 24)
        .await
        .expect("許可したのに開けない");
    assert_eq!(engine.console_holder().await, Some(Actor::Ai));
}

#[tokio::test]
async fn nothing_can_be_done_before_a_connection_is_open() {
    // **繋がっていないのに動く経路があってはいけない**（裏で張ってしまうから）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    assert!(engine.active().await.is_none());
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
        watching
            .borrow()
            .first()
            .map(|o: &sshboard_engine::Opened| o.id.clone()),
        Some("local".to_string())
    );

    assert_eq!(engine.active().await.map(|o| o.id), Some("local".into()));
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
async fn the_same_connection_is_not_opened_twice() {
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
        "同じ相手を二重に開いている"
    );
    assert_eq!(engine.open_connections().await.len(), 1);
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
        engine.disconnect(Actor::Human, None).await.map(|o| o.id),
        Some("local".to_string())
    );
    assert!(engine.active().await.is_none());
    // 2 回目は「何も開いていなかった」。**失敗にしない。**
    assert!(engine.disconnect(Actor::Human, None).await.is_none());
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

// --- 鍵の形式（D28） ---------------------------------------------------------

/// 使い捨てのテスト鍵から、パスフレーズ付きの PPK を作る。
///
/// **本番の鍵は使いません。**`tools/test-server/.key` は毎回捨てられる鍵で、
/// リポジトリにも入っていません。`puttygen` が無ければ `None`。
fn disposable_ppk(dir: &tempfile::TempDir, passphrase: &str) -> Option<PathBuf> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/test-server/.key")
        .canonicalize()
        .ok()?;

    let pass_file = dir.path().join("pass.txt");
    std::fs::write(&pass_file, passphrase).ok()?;
    let ppk = dir.path().join("disposable.ppk");

    let done = std::process::Command::new("puttygen")
        .arg(&source)
        .args(["-O", "private", "--new-passphrase"])
        .arg(&pass_file)
        .arg("-o")
        .arg(&ppk)
        .output()
        .ok()?;

    // パスフレーズを書いた一時ファイルは、**使い終わったらすぐ消す。**
    let _ = std::fs::remove_file(&pass_file);
    done.status.success().then_some(ppk)
}

/// 鍵のパスを書いた接続一覧。
async fn registry_with_key(dir: &tempfile::TempDir, key: &Path) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let toml = format!(
        "version = 1\n\n[[connections]]\nid = \"local\"\nname = \"Local test server\"\n\
         host = \"{HOST}\"\nport = {PORT}\nuser = \"{USER}\"\n\
         fingerprint = \"{}\"\nkey_path = \"{}\"\n",
        known_fingerprint().await,
        toml_string(key)
    );
    std::fs::write(&path, toml).expect("接続一覧を書けない");
    path
}

#[tokio::test]
async fn a_putty_key_connects_without_being_converted_first() {
    // **これが D28 の主張そのものです。**`.ppk` のまま繋がる。
    // puttygen も ssh-add も要らない（人が鍵の形式を意識しない）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let Some(ppk) = disposable_ppk(&dir, "sshboard-test-pass") else {
        println!("puttygen がありません（想定内・飛ばします）");
        return;
    };

    let engine = engine_at(registry_with_key(&dir, &ppk).await);
    let opened = engine
        .connect(Actor::Human, "local", Some("sshboard-test-pass".into()))
        .await
        .expect("PPK のまま繋がらない");

    assert_eq!(opened.id, "local");
    // 本当に使える状態か、1 回サーバーへ触って確かめる。
    engine
        .list_dir(Actor::Human, "/home/probe")
        .await
        .expect("繋がったのに読めない");
}

#[tokio::test]
async fn an_encrypted_putty_key_is_never_tried_without_its_passphrase() {
    // 黙って認証へ行くと「鍵が違います」としか出ず、人は直しようがない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let Some(ppk) = disposable_ppk(&dir, "sshboard-test-pass") else {
        println!("puttygen がありません（想定内・飛ばします）");
        return;
    };

    let engine = engine_at(registry_with_key(&dir, &ppk).await);
    let result = engine.connect(Actor::Human, "local", None).await;

    assert!(
        matches!(result, Err(EngineError::PassphraseNeeded { .. })),
        "{:?}",
        result.map(|open| open.id)
    );
}

// --- ダウンロード（サーバー → 手元） -----------------------------------------

#[tokio::test]
async fn a_remote_file_is_downloaded_byte_for_byte() {
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

    // **自分で足りるようにする。**以前はここが無く、`/home/probe/upload` を
    // 作る別のテストが先に走ったときだけ通っていました。
    // **建てたばかりのサーバーでは落ちます** — 実際に落ちました（Issue #10 の検証中）。
    // 並列で走るので、順番は保証されません。
    engine
        .ensure_dir(Actor::Human, "/home/probe/upload")
        .await
        .expect("置き場を作れない");

    // **文字コードを決めない**（`read_file` と同じ立場）。
    // EUC-JP のログは実在するので、落とした先で化けていては使えない。
    let payload: Vec<u8> = b"\x00\xff\r\n\xa4\xa2\xa4\xa4".to_vec();
    let remote = "/home/probe/upload/download-me.bin";
    engine
        .upload_bytes(Actor::Human, remote, &payload)
        .await
        .expect("置けない");

    let local = dir.path().join("came-back.bin");
    let bytes = engine
        .download_file(Actor::Human, remote, &local, OnConflict::Refuse)
        .await
        .expect("落とせない");

    assert_eq!(bytes, payload.len() as u64);
    assert_eq!(
        std::fs::read(&local).expect("手元に無い"),
        payload,
        "落としたものが元と違う"
    );
}

#[tokio::test]
async fn a_second_download_is_refused_unless_overwriting_was_chosen() {
    // **人の手元を黙って壊さない。**繋がっていても、そこは変わらない。
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

    // **自分で足りるようにする**（上と同じ理由）。
    engine
        .ensure_dir(Actor::Human, "/home/probe/upload")
        .await
        .expect("置き場を作れない");

    let remote = "/home/probe/upload/twice.txt";
    engine
        .upload_bytes(Actor::Human, remote, b"server")
        .await
        .expect("置けない");

    let local = dir.path().join("twice.txt");
    std::fs::write(&local, b"local").expect("手元に書けない");

    let refused = engine
        .download_file(Actor::Human, remote, &local, OnConflict::Refuse)
        .await;
    assert!(matches!(refused, Err(EngineError::Local(_))), "{refused:?}");
    assert_eq!(std::fs::read(&local).expect("読めない"), b"local");

    engine
        .download_file(Actor::Human, remote, &local, OnConflict::Overwrite)
        .await
        .expect("上書きを頼んだのに落とせない");
    assert_eq!(std::fs::read(&local).expect("読めない"), b"server");
}

#[tokio::test]
async fn downloading_something_that_is_not_there_leaves_nothing_behind() {
    // **失敗したのに 0 バイトのファイルが残る**のが一番たちが悪い。
    // 落ちてきたと思って、そのまま次の作業へ進んでしまう。
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

    let local = dir.path().join("never-arrives.bin");
    let result = engine
        .download_file(
            Actor::Human,
            "/home/probe/upload/no-such-file-at-all",
            &local,
            OnConflict::Refuse,
        )
        .await;

    assert!(result.is_err(), "無いものが落ちてきた");
    assert!(
        !local.exists(),
        "落ちなかったのに手元にファイルが残っている"
    );
}

// --- 複数の接続（D25） -------------------------------------------------------

#[tokio::test]
async fn several_connections_stay_open_at_once_and_all_of_them_are_listed() {
    // **切らずに次へ移れること**が要（D25）。1 本ずつ切って繋ぎ直す道具は使われない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry_pair(&dir).await);

    engine
        .connect(Actor::Human, "first", None)
        .await
        .expect("1 本目");
    engine
        .connect(Actor::Human, "second", None)
        .await
        .expect("2 本目");

    let open = engine.open_connections().await;
    assert_eq!(
        open.len(),
        2,
        "**開いたものが一覧に出ていない**（裏に持っている）"
    );
    let ids: Vec<_> = open.iter().map(|o| o.id.as_str()).collect();
    assert!(ids.contains(&"first") && ids.contains(&"second"));
}

#[tokio::test]
async fn opening_a_second_connection_points_operations_at_it() {
    // **開いたのに何も向いていない、を作らない。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry_pair(&dir).await);

    engine
        .connect(Actor::Human, "first", None)
        .await
        .expect("1 本目");
    engine
        .connect(Actor::Human, "second", None)
        .await
        .expect("2 本目");

    assert_eq!(engine.active().await.map(|o| o.id), Some("second".into()));

    // 戻れること。**タブを押す操作。**
    engine.focus("first").await.expect("戻れない");
    assert_eq!(engine.active().await.map(|o| o.id), Some("first".into()));
}

#[tokio::test]
async fn closing_the_focused_connection_moves_the_focus_to_one_that_is_still_open() {
    // 宛先が空のまま接続だけ開いている、という読めない状態を作らない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry_pair(&dir).await);
    engine
        .connect(Actor::Human, "first", None)
        .await
        .expect("1 本目");
    engine
        .connect(Actor::Human, "second", None)
        .await
        .expect("2 本目");

    engine.disconnect(Actor::Human, Some("second")).await;

    assert_eq!(engine.open_connections().await.len(), 1);
    assert_eq!(
        engine.active().await.map(|o| o.id),
        Some("first".into()),
        "宛先が空のまま接続が残っている"
    );
}

#[tokio::test]
async fn focusing_something_that_is_not_open_is_refused() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    // **`NotConnected` とは分けます**（Issue #8）。
    //
    // 実機から報告された赤帯の文言は `NotConnected` の表示文でしたが、
    // **同じ文言を返す道が 2 本**あり（宛先が決まっていない／押したタブが
    // 開いていない）、**画面写真からどちらか判別できませんでした。**
    // 分けたので、次に踏まれたら文言で確定します。
    assert!(matches!(
        engine.focus("nothing-here").await,
        Err(EngineError::NotOpen { .. })
    ));
}

#[tokio::test]
async fn operations_follow_the_focus_rather_than_the_most_recent_connection() {
    // **タブを押したら、その相手に効く。**ここがずれると、
    // 本番へ送るつもりで開発へ送る事故になる。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry_pair(&dir).await);
    engine
        .connect(Actor::Human, "first", None)
        .await
        .expect("1 本目");
    engine
        .connect(Actor::Human, "second", None)
        .await
        .expect("2 本目");

    engine.focus("first").await.expect("戻れない");
    // 宛先が生きていれば、そこへ操作が通る。
    let listed = engine
        .list_dir(Actor::Human, "/home/probe")
        .await
        .expect("宛先へ操作が通らない");
    assert!(!listed.is_empty());
}

// --- コマンドを打つ経路 --------------------------------------------------------

#[tokio::test]
async fn a_command_runs_through_the_engine_and_comes_back_whole() {
    // `Engine::exec` は**エンジン層のテストが 1 本も無かった**（SSH 層にはあった）。
    // 端末（D29）はこの経路の上に載るので、ここを押さえておく。
    //
    // **当たり障りのないコマンドだけを打つ。**サーバーの状態を変えない。
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

    for command in ["cal", "date", "id -un", "df -h /"] {
        let ran = engine
            .exec(Actor::Human, command)
            .await
            .unwrap_or_else(|error| panic!("{command} が打てない: {error}"));
        println!("--- $ {command} ---\n{}", ran.out);
        if !ran.err.trim().is_empty() {
            println!("--- $ {command} (stderr) ---\n{}", ran.err);
        }
        assert!(ran.succeeded(), "{command} が失敗した: {ran:?}");
        assert!(!ran.out.trim().is_empty(), "{command} が何も返さない");
    }
}

// --- 権限を上げて root 領域を読む（D48 / Issue #19） ---------------------------

/// **パスワードを聞かれる `sudo` を持った利用者**で登録する。
///
/// 実機の状態がこれでした ——
///
/// ```text
/// $ sudo -n true
/// sudo: パスワードが必要です
/// ```
///
/// **`sudoers` の道が閉じているサーバー**です。テスト用サーバーの `pw` に
/// `NOPASSWD` **無し**の sudo を与えて、同じ状態を作ってあります。
async fn registry_that_asks(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let toml = format!(
        "version = 1\n\n[[connections]]\nid = \"sudoer\"\nname = \"Asks for a password\"\n\
         host = \"{HOST}\"\nport = {PORT}\nuser = \"pw\"\n\
         fingerprint = \"{}\"\nbecome = \"ask\"\n",
        known_fingerprint().await
    );
    std::fs::write(&path, toml).expect("接続一覧を書けない");
    path
}

/// **root しか読めないログ**を読む操作を、人が書いておく。
fn operations_reading_a_root_only_log(dir: &tempfile::TempDir) {
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"read-maillog\"\n\
         run = \"sudo cat /var/log/maillog\"\n\
         description = \"root しか読めないログを読む\"\nmax_per_hour = 5\n",
    )
    .expect("一覧を書けない");
}

/// テスト用サーバーの `pw` のパスワード。**使い捨てのコンテナのものです。**
/// 製品にも実機にも一切関係しません（`tools/test-server/Dockerfile` に在ります）。
const PW_PASSWORD: &str = "sshboard-test-password";

#[tokio::test]
async fn a_root_only_log_can_be_read_once_the_person_types_the_password() {
    // **#19 が問うているのはこれです。**
    //
    // > `operations.toml` は「**どのコマンドを走らせてよいか**」を解きますが、
    // > 「**どうやって権限を得るか**」は解いていません
    //
    // > **AI 側から root 領域を読む道が、現時点でゼロです。**
    //
    // `/var/log/maillog` は `root:root 600`。**実機の `/var/log/maillog` と同じ状態**で、
    // ログインした利用者では 1 バイトも読めません。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    operations_reading_a_root_only_log(&dir);
    let engine = engine_at(registry_that_asks(&dir).await);
    engine
        .connect(Actor::Human, "sudoer", Some(PW_PASSWORD.into()))
        .await
        .expect("繋がらない");

    // **まず読めないことを確かめる。**これが直したい状態です。
    let blind = engine
        .exec(Actor::Ai, "cat /var/log/maillog")
        .await
        .expect("コマンドが打てない");
    assert!(
        !blind.succeeded(),
        "**root しか読めないはずのログが素で読めている**: {blind:?}"
    );

    // 1. AI が呼ぶ。**人に尋ねて、そこで止まる。**
    let stopped = engine.run_operation(Actor::Ai, "read-maillog").await;
    assert!(
        matches!(stopped, Err(EngineError::ConsoleApprovalNeeded)),
        "承認なしで走ろうとしている: {stopped:?}"
    );

    // 2. 画面へ出る問い。**打つものがそのまま載り、パスワードを聞くと分かる。**
    let asked = engine.operation_request().expect("問いが立っていない");
    assert_eq!(asked.id, "read-maillog");
    assert_eq!(
        asked.runs, "sudo -S -p '' cat /var/log/maillog",
        "**当てはめる前を見せている**（人は `-S` が付くことを知らないまま聞かれる）"
    );
    assert!(asked.needs_secret, "パスワードを聞くと分かっていない");

    // 3. 人がその場で入れる。**保存しません。**
    engine
        .answer_operation(Actor::Human, true, Some(PW_PASSWORD.into()))
        .await
        .expect("人が許せない");

    // 4. **読めます。**
    let ran = engine
        .run_operation(Actor::Ai, "read-maillog")
        .await
        .expect("走らない");
    assert!(
        ran.succeeded(),
        "**root 領域へ届いていない**: {ran:?}（stderr: {})",
        ran.err
    );
    assert!(
        ran.out.contains("postfix/smtpd"),
        "中身が返っていない: {ran:?}"
    );

    // 5. **使い切り。**もう一度呼んだら、また尋ねる。
    let again = engine.run_operation(Actor::Ai, "read-maillog").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "**秘密が残っている**: {again:?}"
    );
}

#[tokio::test]
async fn without_the_password_it_fails_instead_of_hanging() {
    // **`sudo -S` に標準入力を渡さないと、返ってきません**（exec に端末はありません）。
    // 空で閉じれば、その場で落ちて人に伝わります。
    // **返ってこない**のがいちばん困る壊れ方なので、ここを見張ります。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    operations_reading_a_root_only_log(&dir);
    let engine = engine_at(registry_that_asks(&dir).await);
    engine
        .connect(Actor::Human, "sudoer", Some(PW_PASSWORD.into()))
        .await
        .expect("繋がらない");

    let _ = engine.run_operation(Actor::Ai, "read-maillog").await;
    // **許すが、何も入れない。**人が空のまま［許可］を押した状態です。
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");

    let ran = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        engine.run_operation(Actor::Ai, "read-maillog"),
    )
    .await
    .expect("**返ってきません**（標準入力を閉じていない）")
    .expect("コマンドが打てない");

    assert!(!ran.succeeded(), "パスワード無しで通っている: {ran:?}");
    assert!(
        !ran.out.contains("postfix/smtpd"),
        "**読めてしまっている**: {ran:?}"
    );
}

#[tokio::test]
async fn the_password_never_reaches_the_screen() {
    // **端末の面（D41）へ流れたら、画面にも巻き戻しにも残ります。**
    // 出てよいのは `$ sudo -S -p '' …` までです。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    operations_reading_a_root_only_log(&dir);
    let engine = engine_at(registry_that_asks(&dir).await);
    engine
        .connect(Actor::Human, "sudoer", Some(PW_PASSWORD.into()))
        .await
        .expect("繋がらない");

    let mut watching = engine.stream().subscribe_raw();

    let _ = engine.run_operation(Actor::Ai, "read-maillog").await;
    engine
        .answer_operation(Actor::Human, true, Some(PW_PASSWORD.into()))
        .await
        .expect("人が許せない");
    engine
        .run_operation(Actor::Ai, "read-maillog")
        .await
        .expect("走らない");

    // 流れてきたものを全部集める。
    let mut shown = String::new();
    while let Ok(Ok(chunk)) =
        tokio::time::timeout(std::time::Duration::from_millis(300), watching.recv()).await
    {
        shown.push_str(&String::from_utf8_lossy(&chunk));
    }

    assert!(
        shown.contains("sudo -S -p ''"),
        "打ったものが画面に出ていない: {shown:?}"
    );
    assert!(
        !shown.contains(PW_PASSWORD),
        "**パスワードが画面へ出ている**: {shown:?}"
    );
}

// --- 端末のロックと停止（D29） -------------------------------------------------

async fn engine_connected(dir: &tempfile::TempDir) -> Engine {
    let engine = engine_at(registry(dir, &[]).await);
    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");
    engine
}

#[tokio::test]
async fn only_the_side_holding_the_console_can_type_into_it() {
    // **同時に触れるのは 1 人**（D29）。
    // 人と AI が交互に打つと、**どちらの意図でもない文字列**がシェルへ入る。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_connected(&dir).await;

    engine
        .console_open(Actor::Human, 80, 24)
        .await
        .expect("開けない");
    assert_eq!(engine.console_holder().await, Some(Actor::Human));

    let refused = engine.console_type(Actor::Ai, b"echo nope\n").await;
    assert!(
        matches!(refused, Err(EngineError::ConsoleHeldByOther { .. })),
        "**握っていない側が打ててしまう**: {refused:?}"
    );

    engine
        .console_type(Actor::Human, b"echo mine\n")
        .await
        .expect("握っている側が打てない");
}

#[tokio::test]
async fn a_person_can_always_take_the_console_back() {
    // **人の解除が常に勝つ**（D29）。AI が握ったまま離さない状態を作らない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_connected(&dir).await;

    ai_console_open(&engine).await;

    // AI が握っていても、人は取り返せる。
    engine
        .console_take(Actor::Human)
        .await
        .expect("取り返せない");
    assert_eq!(engine.console_holder().await, Some(Actor::Human));
    engine
        .console_type(Actor::Human, b"echo taken\n")
        .await
        .expect("取り返したのに打てない");

    // **逆は勝たない。**AI は人から奪えない。
    // **D42 以降、AI にできるのは「頼む」ことだけ**です（人が答えるまで握れません）。
    let refused = engine.console_take(Actor::Ai).await;
    assert!(
        matches!(refused, Err(EngineError::ConsoleApprovalNeeded)),
        "**AI が人から奪えてしまう**: {refused:?}"
    );
    assert_eq!(
        engine.console_holder().await,
        Some(Actor::Human),
        "頼んだだけで握りが動いている"
    );
}

#[tokio::test]
async fn stopping_the_console_always_works_and_frees_it() {
    // **止まらない停止ボタンは、無い方がまし**（D29）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_connected(&dir).await;

    ai_console_open(&engine).await;
    let _ = engine.console_stop(Actor::Human).await;
    assert_eq!(
        engine.console_holder().await,
        None,
        "止めたのに握られたまま"
    );

    // 止めたあとは、誰でも開き直せる。**止めたら二度と使えない、にしない。**
    engine
        .console_open(Actor::Human, 80, 24)
        .await
        .expect("止めたあとに開けない");
    assert_eq!(engine.console_holder().await, Some(Actor::Human));
    let _ = engine.console_stop(Actor::Human).await;
}

#[tokio::test]
async fn a_console_says_which_connection_it_belongs_to() {
    // **一番危ない食い違い。**サーバー A で端末を開き、タブで B へ切り替えて打つと、
    // 打鍵は A のシェルへ行く。**画面は B を向いているのに。**
    //
    // 端末が「どの接続のものか」を覚えていないと、人も AI も気づけない。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(registry_pair(&dir).await);
    engine
        .connect(Actor::Human, "first", None)
        .await
        .expect("繋がらない");
    engine
        .connect(Actor::Human, "second", None)
        .await
        .expect("繋がらない");

    // いまの宛先は second。**first を宛先にしてから端末を開く。**
    engine.focus("first").await.expect("向けられない");
    engine
        .console_open(Actor::Human, 80, 24)
        .await
        .expect("開けない");
    assert_eq!(
        engine.console_connection().await.as_deref(),
        Some("first"),
        "端末がどの接続のものか覚えていない"
    );

    // タブを second へ移す。**端末は first のまま**でなければならない。
    engine.focus("second").await.expect("向けられない");
    assert_eq!(
        engine.console_connection().await.as_deref(),
        Some("first"),
        "宛先を変えたら端末まで別の接続を指してしまった"
    );

    // **黙って乗り換えない。**別の接続で開こうとしたら、どこで開いているかを言って断る。
    let refused = engine.console_open(Actor::Human, 80, 24).await;
    assert!(
        matches!(refused, Err(EngineError::ConsoleOnOtherConnection { ref id }) if id == "first"),
        "別の接続へ黙って乗り換えた: {refused:?}"
    );

    let _ = engine.console_stop(Actor::Human).await;
    // 止めたあとは、いまの宛先で開ける。
    engine
        .console_open(Actor::Human, 80, 24)
        .await
        .expect("止めたあとに開けない");
    assert_eq!(engine.console_connection().await.as_deref(), Some("second"));
    let _ = engine.console_stop(Actor::Human).await;
}

/// 移動しても宛先が落ちないこと（Issue #8）。
///
/// > ファイル画面で移動すると接続先の表示が消え、繋がっているのに
/// > 「まだ繋がっていません」と出続ける
///
/// **`list_dir` は `active` が要ります。**移動のたびに呼ぶので、
/// ここが落ちると画面が「繋がっていない」に見えます。
#[tokio::test]
async fn moving_around_does_not_lose_the_active_session() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    // `up.sh` が作る使い捨ての鍵。**実機の鍵とは無関係。**
    let key = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/test-server/.key"
    ));
    if !key.exists() {
        println!("使い捨ての鍵がありません（想定内・飛ばします）");
        return;
    }
    let engine = engine_at(registry_with_key(&dir, &key).await);

    engine
        .connect(Actor::Human, "local", None)
        .await
        .expect("繋がらない");

    // **何度も移動する。**1 回目は通って 2 回目で落ちる、を捕まえるため。
    for path in [".", "app", "app/logs", ".", "app"] {
        engine
            .list_dir(Actor::Human, path)
            .await
            .unwrap_or_else(|error| panic!("{path} を読めない: {error}"));
        assert!(
            engine.active().await.is_some(),
            "{path} を読んだあと、宛先が落ちている"
        );
    }
}

#[tokio::test]
async fn what_a_purpose_built_tool_runs_shows_up_on_the_screen() {
    // **実機の指摘**（Issue #14 の 2 つ目）:
    //
    // > 用途別ツールは実際にサーバーでコマンドを走らせているので、
    // > **端末に出ないほうが不自然**です
    //
    // `exec` は共有している出力へ 1 バイトも流しておらず、
    // **AI がサーバーで何を見たのかを、人は追えません**でした。
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

    // **画面の側で待ち受けてから走らせる。**
    let mut watching = engine.stream().subscribe_raw();

    let ran = engine
        .runtime_versions(Actor::Ai)
        .await
        .expect("用途別ツールが走らない");

    // 画面へ届いた分を集める。**打ったものと、返ってきたものの両方。**
    let mut shown = String::new();
    while let Ok(chunk) = watching.try_recv() {
        shown.push_str(&String::from_utf8_lossy(&chunk));
    }

    assert!(
        shown.contains('$'),
        "**打ったコマンドが画面に出ていない。**\
         人は AI が何を走らせたのか追えません:\n{shown}"
    );
    assert!(
        !ran.out.is_empty() && shown.contains(ran.out.lines().next().unwrap_or("")),
        "**返ってきたものが画面に出ていない**:\n{shown}"
    );
}

#[tokio::test]
async fn handing_the_console_over_keeps_the_same_shell() {
    // **実機の指摘**（Issue #21）—— 握りを渡すと別のシェルが開いていました。
    //
    // > 人が `su -` して root になったあと AI へ渡すと、
    // > **`Last login` が途中で出て、プロンプトが元の利用者に戻る**
    //
    // `su` も、カレントディレクトリも、環境変数も、実行中のジョブも消えます。
    // そして **`read_stream` は 2 本の出力を区切り無しに 1 本に見せます。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_connected(&dir).await;

    // 人が開いて、下ごしらえをする。
    let first = engine
        .console_open(Actor::Human, 80, 24)
        .await
        .expect("人が開けない");
    assert_eq!(first, sshboard_engine::ConsoleOpened::Fresh);

    // AI が握る（D42 の承認を通す）。
    let asked = engine.console_open(Actor::Ai, 80, 24).await;
    assert!(
        matches!(asked, Err(EngineError::ConsoleApprovalNeeded)),
        "許可なく握れてしまう: {asked:?}"
    );
    engine
        .console_answer(Actor::Human, true)
        .await
        .expect("人が許せない");

    let second = engine
        .console_open(Actor::Ai, 80, 24)
        .await
        .expect("許可したのに開けない");

    // **ここが本題。**新しく立てず、受け取ること。
    assert_eq!(
        second,
        sshboard_engine::ConsoleOpened::TookOver,
        "**握りを渡したのに、別のシェルが開いている。**\
         人が `su -` した状態も、カレントディレクトリも消えます"
    );
    assert_eq!(engine.console_holder().await, Some(Actor::Ai));
}
