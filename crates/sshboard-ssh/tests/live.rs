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

    // 1 回目で指紋を知る。**人が確かめて登録する流れと同じ。**
    let Err(SshError::UntrustedHost { seen, .. }) =
        SshSession::connect(&target(None), &Auth::Agent, Band::new()).await
    else {
        panic!("初見のホストを通している");
    };

    // 2 回目は登録済みとして繋ぐ。
    let session = SshSession::connect(&target(Some(&seen.fingerprint)), &Auth::Agent, Band::new())
        .await
        .expect("登録済みの指紋で繋がらない");

    assert_eq!(session.host_key().fingerprint, seen.fingerprint);

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

    let Err(SshError::UntrustedHost { seen, .. }) =
        SshSession::connect(&target(None), &Auth::Agent, Band::new()).await
    else {
        panic!("初見のホストを通している");
    };

    let band = Band::new();
    let mut subscriber = band.subscribe();
    let session = SshSession::connect(&target(Some(&seen.fingerprint)), &Auth::Agent, band)
        .await
        .expect("繋がらない");

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
