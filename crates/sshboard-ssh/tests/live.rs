//! 実際に繋ぐテスト。
//!
//! **サーバーが無い環境でも走ります**（product-baseline §4）。
//! 建てるには `sh tools/test-server/up.sh`。

use sshboard_band::{Actor, Band};
use sshboard_ssh::{Auth, SshError, SshSession, Target};

const HOST: &str = "127.0.0.1";
const PORT: u16 = 2222;
const USER: &str = "probe";

fn target(pinned: Option<&str>) -> Target {
    Target {
        host: HOST.to_owned(),
        port: PORT,
        user: USER.to_owned(),
        pinned_fingerprint: pinned.map(str::to_owned),
        // **初見でも通るよう、known_hosts の代わりに指紋を渡す形で試す。**
        known_hosts: String::new(),
        // **既定は AI 拒否。**書き込みを試すテストだけが明示的に開ける。
        write_scope: sshboard_ssh::WriteScope::default(),
    }
}

/// テスト用サーバーが建っていなければ、そのテストは飛ばす。
async fn server_is_up() -> bool {
    tokio::net::TcpStream::connect((HOST, PORT)).await.is_ok()
}

#[tokio::test]
async fn a_first_time_host_is_refused_even_though_the_connection_succeeded() {
    // **ここが D6 の要。**繋がることと、信用してよいことは別。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let result = SshSession::connect(&target(None), &Auth::Agent, Band::new()).await;

    match result {
        Err(SshError::UntrustedHost { seen, .. }) => {
            assert!(seen.fingerprint.starts_with("SHA256:"), "実際: {seen:?}");
            assert!(!seen.algorithm.is_empty());
        }
        other => panic!("初見のホストを通している: {other:?}", other = other.err()),
    }
}

#[tokio::test]
async fn a_pinned_host_connects_and_runs_a_command() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    // 初見で 1 回拒否されたときの指紋を使う。**人が確かめて登録する流れと同じ。**
    let expected = known_fingerprint().await.clone();
    let session = trusted_session(Band::new()).await;

    assert_eq!(session.host_key().fingerprint, expected);

    let out = session
        .exec(Actor::Human, "echo sshboard-ok")
        .await
        .expect("コマンドが通らない");
    assert_eq!(out.trim(), "sshboard-ok");
}

#[tokio::test]
async fn a_wrong_pin_is_refused() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let result = SshSession::connect(
        &target(Some("SHA256:definitely-not-it")),
        &Auth::Agent,
        Band::new(),
    )
    .await;

    assert!(
        matches!(result, Err(SshError::UntrustedHost { .. })),
        "食い違う指紋で通している"
    );
}

#[tokio::test]
async fn every_command_reaches_the_band_before_it_answers() {
    // **見えないまま実行しない**（D16）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let mut subscriber = band.subscribe();
    let session = trusted_session(band).await;

    let running = tokio::spawn(async move { session.exec(Actor::Ai, "echo hi").await });
    let event = subscriber.recv().await.expect("帯へ出ていない");

    assert_eq!(event.line().actor(), Actor::Ai);
    assert_eq!(event.line().text(), "$ echo hi");
    event.ack();

    running
        .await
        .expect("パニック")
        .expect("コマンドが通らない");
}

/// テスト用サーバーの指紋。**1 回だけ調べて使い回す。**
///
/// 毎回調べると、テストの本数 × 2 本の接続が同時に飛び、
/// sshd の `MaxStartups`（既定 10）に弾かれる。**実際に弾かれた。**
static FINGERPRINT: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

async fn known_fingerprint() -> &'static String {
    FINGERPRINT
        .get_or_init(|| async {
            match SshSession::connect(&target(None), &Auth::Agent, Band::new()).await {
                Err(SshError::UntrustedHost { seen, .. }) => seen.fingerprint,
                Ok(_) => panic!("初見のホストを通している"),
                // **接続失敗を「通した」と言わない。**エラー文は嘘をつかない
                Err(other) => panic!("繋げません: {other}"),
            }
        })
        .await
}

/// 登録済みとして繋ぐ。
async fn trusted_session(band: Band) -> SshSession {
    SshSession::connect(&target(Some(known_fingerprint().await)), &Auth::Agent, band)
        .await
        .expect("登録済みの指紋で繋がらない")
}

#[tokio::test]
async fn sftp_and_exec_run_on_the_same_session() {
    // **2 本目の接続を張らない**（PRD §4-1）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = trusted_session(Band::new()).await;

    let out = session
        .exec(Actor::Human, "whoami")
        .await
        .expect("exec が通らない");
    let entries = session
        .list_dir(Actor::Human, "/home/probe/app/logs")
        .await
        .expect("ls が通らない");

    assert_eq!(out.trim(), "probe");
    assert!(
        entries.iter().any(|e| e.name == "app.log"),
        "実際: {entries:?}"
    );
}

#[tokio::test]
async fn a_euc_jp_log_comes_back_as_bytes_not_mangled_text() {
    // **ここで文字コードを決めない**（Issue 002）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = trusted_session(Band::new()).await;
    let bytes = session
        .read_file(Actor::Human, "/var/log/japanese-euc.log")
        .await
        .expect("読めない");

    assert!(!bytes.is_empty());
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "EUC-JP のはずが UTF-8 として通っている（テスト用サーバーの前提が崩れている）"
    );
}

#[tokio::test]
async fn a_root_only_log_is_refused_rather_than_returning_nothing() {
    // **黙って空を返さない。**読めないことが分かる形で返る（D20 の前提）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = trusted_session(Band::new()).await;
    let result = session.read_file(Actor::Human, "/var/log/maillog").await;

    assert!(result.is_err(), "root しか読めないログが読めてしまっている");
}

#[tokio::test]
async fn following_a_log_feeds_both_faces_and_stops_when_the_human_stops_it() {
    // Issue 005 を実機で。**GUI は色付き / MCP は素。**
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    use sshboard_stream::OutputStream;
    use std::sync::Arc;

    let session = trusted_session(Band::new()).await;
    let stream = Arc::new(OutputStream::new());
    let mut raw = stream.subscribe_raw();
    let mut plain = stream.subscribe_plain();

    let following = {
        let stream = Arc::clone(&stream);
        tokio::spawn(async move {
            session
                .follow(Actor::Human, "/home/probe/app/logs/app.log", 5, stream)
                .await
        })
    };

    // 生の側には色が残り、素の側には残らない。
    let raw_chunk = tokio::time::timeout(std::time::Duration::from_secs(10), raw.recv())
        .await
        .expect("生の側へ来ない")
        .expect("閉じている");
    let plain_chunk = tokio::time::timeout(std::time::Duration::from_secs(10), plain.recv())
        .await
        .expect("素の側へ来ない")
        .expect("閉じている");

    assert!(raw_chunk.contains(&0x1b), "GUI 側の色が落ちている");
    assert!(
        !plain_chunk.contains('\x1b'),
        "MCP 側に ANSI が混ざっている: {plain_chunk:?}"
    );

    // 人が止めたら、その場で追うのをやめる（PRD §4-3）。
    stream.stop();
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(15), following).await;
    assert!(stopped.is_ok(), "止めたのに追い続けている");
}

// --- 書き込み（D22） ---------------------------------------------------------
// **AI の書き込みは囲いの中だけ。人は制限しない**（PRD §3）。

/// 書き込み許可つきで繋ぐ。
async fn writable_session(roots: &[&str]) -> SshSession {
    let scope = sshboard_ssh::WriteScope::under(roots).expect("囲いを作れない");
    let mut target = target(Some(known_fingerprint().await));
    target.write_scope = scope;
    SshSession::connect(&target, &Auth::Agent, Band::new())
        .await
        .expect("登録済みの指紋で繋がらない")
}

#[tokio::test]
async fn a_human_upload_lands_on_the_server_and_reads_back_byte_for_byte() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = writable_session(&["/home/probe/upload"]).await;
    let path = "/home/probe/upload/human.bin";
    // 改行も非 ASCII も混ぜる。**テキストとして触ると壊れる中身で確かめる。**
    let payload: Vec<u8> = b"\x00\x01line1\nline2\r\n\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e".to_vec();

    session
        .ensure_dir(Actor::Human, "/home/probe/upload")
        .await
        .expect("ディレクトリを作れない");
    session
        .upload(Actor::Human, path, &payload)
        .await
        .expect("上げられない");

    let back = session
        .read_file(Actor::Human, path)
        .await
        .expect("読み戻せない");
    assert_eq!(back, payload, "上げたものと落としたものが違う");
}

#[tokio::test]
async fn an_ai_upload_outside_the_allowed_directory_is_refused_before_it_touches_the_server() {
    // **ここが D22 の要。**許可の外は、サーバーへ届く前に断る。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = writable_session(&["/home/probe/upload"]).await;
    let outside = "/home/probe/elsewhere.bin";

    let refused = session.upload(Actor::Ai, outside, b"x").await;
    assert!(
        matches!(refused, Err(SshError::WriteRefused(_))),
        "囲いの外へ AI が書けてしまっている: {refused:?}"
    );

    // **本当に届いていないこと**を、サーバー側を見て確かめる。
    let listed = session
        .list_dir(Actor::Human, "/home/probe")
        .await
        .expect("一覧が取れない");
    assert!(
        !listed.iter().any(|e| e.name == "elsewhere.bin"),
        "断ったはずのファイルがサーバーにある"
    );
}

#[tokio::test]
async fn an_ai_upload_inside_the_allowed_directory_goes_through() {
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = writable_session(&["/home/probe/upload"]).await;
    session
        .ensure_dir(Actor::Ai, "/home/probe/upload/ai")
        .await
        .expect("囲いの中なのに作れない");
    session
        .upload(Actor::Ai, "/home/probe/upload/ai/ok.txt", b"ok")
        .await
        .expect("囲いの中なのに上げられない");

    let back = session
        .read_file(Actor::Human, "/home/probe/upload/ai/ok.txt")
        .await
        .expect("読み戻せない");
    assert_eq!(back, b"ok");
}

#[tokio::test]
async fn a_connection_without_a_write_scope_lets_the_human_write_but_not_the_ai() {
    // **既定は AI 拒否。**設定を忘れた接続で AI が書けてしまうのが一番まずい。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let session = trusted_session(Band::new()).await; // write_scope は既定＝Denied
    let path = "/home/probe/upload/default.txt";

    session
        .ensure_dir(Actor::Human, "/home/probe/upload")
        .await
        .expect("人が作れない");
    session
        .upload(Actor::Human, path, b"human")
        .await
        .expect("人が制限されている");

    let refused = session.upload(Actor::Ai, path, b"ai").await;
    assert!(
        matches!(refused, Err(SshError::WriteRefused(_))),
        "囲い未設定で AI が書けてしまっている: {refused:?}"
    );
}

#[tokio::test]
async fn an_upload_reaches_the_band_before_it_is_written() {
    // **見えないまま書かない**（PRD §4-2・D16）。
    if !server_is_up().await {
        println!("テスト用サーバーが建っていません（想定内・飛ばします）");
        return;
    }

    let band = Band::new();
    let mut watching = band.subscribe();

    let scope = sshboard_ssh::WriteScope::under(["/home/probe/upload"]).expect("囲い");
    let mut wanted = target(Some(known_fingerprint().await));
    wanted.write_scope = scope;
    let session = SshSession::connect(&wanted, &Auth::Agent, band)
        .await
        .expect("繋がらない");

    let writing = tokio::spawn(async move {
        session
            .ensure_dir(Actor::Ai, "/home/probe/upload")
            .await
            .expect("作れない");
        session
            .upload(Actor::Ai, "/home/probe/upload/seen.txt", b"seen")
            .await
    });

    // **書き込みが終わる前に**帯へ出ていること。受け取りを返さない限り先へ進まない。
    let mut seen = Vec::new();
    for _ in 0..2 {
        let event = tokio::time::timeout(std::time::Duration::from_secs(10), watching.recv())
            .await
            .expect("帯へ出ていない")
            .expect("帯が閉じている");
        assert_eq!(event.line().actor(), Actor::Ai);
        seen.push(event.line().text().to_string());
        event.ack();
    }

    let written = writing.await.expect("パニック").expect("上げられない");
    assert_eq!(written, 4);
    assert!(
        seen.iter().any(|l| l.contains("seen.txt")),
        "書き込みが帯に出ていない: {seen:?}"
    );
}
