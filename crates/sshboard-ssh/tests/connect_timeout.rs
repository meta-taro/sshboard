//! **繋がらない相手を、いつまでも待たない**（実運用の指摘・2026-09-28）。
//!
//! 実機からの報告 ——
//!
//! > 鍵パスを Windows のパスに直したうえで、もう一度 MCP の connect を呼びました。
//! > **120 秒たっても返らず**、そのあとに呼んだ pending_status まで 120 秒以上返りません。
//!
//! **MCP のサーバーは詰まっていません。**別の口から `pending_status` を呼ぶと
//! **0.0 秒で返ります**（実測）。詰まっているのは**呼んでいる側**で、
//! **1 本の呼びを待っている間、その席は次を送れません。**
//!
//! つまり **`connect` が返らないこと自体**が本体です。
//! そして `connect` には、**時間切れが 1 つも設けられていませんでした** ——
//! `inactivity_timeout: None`（D25。開いたまま置く道具なので、**繋がったあと**は正しい）、
//! **TCP を張る所には何も無し** → OS の既定（Windows は 20 秒 × 再試行）。
//!
//! **「繋がったあと切らない」と「繋ぎに行く時間を区切る」は、別のこと**です。
//! 前者を理由に後者を無くすと、**相手が居ない回に、人も AI も 2 分沈黙します。**
//!
//! `192.0.2.1` は **TEST-NET-1**（RFC 5737）。誰のものでもなく、応答しません。

use std::time::{Duration, Instant};

use sshboard_band::Band;
use sshboard_diag::Diagnostics;
use sshboard_ssh::{Auth, SshSession, Target, WriteScope};

/// **待つ上限。**これを超えたら、試験としては落ちです。
const MUST_GIVE_UP_WITHIN: Duration = Duration::from_secs(25);

#[tokio::test]
async fn a_host_that_never_answers_is_given_up_on() {
    let target = Target {
        id: Some("nowhere".into()),
        host: "192.0.2.1".into(),
        port: 22,
        user: "probe".into(),
        pinned_fingerprint: None,
        known_hosts: String::new(),
        // **AI は書けない**（既定・D22）。ここでは繋がらないので使われません。
        write_scope: WriteScope::Denied,
    };

    let started = Instant::now();
    let result = SshSession::connect(&target, &Auth::Agent, Band::new(), &Diagnostics::new()).await;
    let took = started.elapsed();

    assert!(result.is_err(), "応答しない相手に繋がっています");
    assert!(
        took < MUST_GIVE_UP_WITHIN,
        "**諦めるのが遅すぎます**: {took:?}（上限 {MUST_GIVE_UP_WITHIN:?}）。\
         この間、呼んだ席は次の呼びを送れません"
    );
}
