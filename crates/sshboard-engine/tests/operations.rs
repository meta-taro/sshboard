//! 状態を変える操作の関門（D45 / D47 / Issue #17）。**サーバーを一切使いません。**
//!
//! ここで見たいのは「**走る前に止まるか**」だからです。走ってから止めるのでは、
//! 止めたことになりません。
//!
//! > **「人が見ている」を安全の根拠にしているのを、「人が承認した」に移す**
//!
//! 見張るのは 7 つ。
//!
//! 1. **既定は空。**人が書くまで 1 本も走らない
//! 2. **AI は、人が許可するまで走らせられない**
//! 3. **問いには、実際に打つものが載る**（何を許すのか読めないと答えられない）
//! 4. **AI は自分で自分を許可できない**
//! 5. **断ったら走らない**
//! 6. **許可は使い切り。**1 回の許可で 1 回だけ
//! 7. **人はいつでも走らせられる**（承認も上限もかからない）

use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_stream::OutputStream;

/// 接続一覧だけ置いた Engine。**1 件も繋いでいません。**
fn engine_in(dir: &tempfile::TempDir) -> Engine {
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

/// 人が書いた一覧を置く。**製品は既定を 1 本も持ちません。**
fn allow(dir: &tempfile::TempDir) {
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"reload-httpd\"\n\
         run = \"systemctl reload httpd\"\n\
         description = \"設定を読み直させる（無停止）\"\nmax_per_hour = 2\n",
    )
    .expect("一覧を書けない");
}

#[tokio::test]
async fn nothing_runs_until_a_person_writes_the_file() {
    // **既定は空**（D3 と同じ立場）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(refused, Err(EngineError::OperationNotAllowed { .. })),
        "一覧が無いのに通っている: {refused:?}"
    );
}

#[tokio::test]
async fn the_ai_cannot_run_one_until_the_person_allows_it() {
    // **これが芯です。**書いてあるだけでは走りません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(refused, Err(EngineError::ConsoleApprovalNeeded)),
        "**承認なしで走ろうとしている**: {refused:?}"
    );
}

#[tokio::test]
async fn the_question_carries_what_would_actually_run() {
    // **何が走るのか読めないと、人は答えられません。**
    // 「restart-httpd を許可しますか」だけでは、実際に何が打たれるか分かりません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let asked = engine.operation_request().expect("問いが立っていない");
    assert_eq!(asked.id, "reload-httpd");
    assert_eq!(asked.runs, "systemctl reload httpd");
    // **繋がっていないので、上げ方も分かりません。**聞かないのが正しい。
    assert!(!asked.needs_secret, "秘密を求めている");
}

#[tokio::test]
async fn a_secret_is_never_carried_in_the_question() {
    // **問いは画面に出ます。**秘密が載ったら、画面と巻き戻しの両方に残ります。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let asked = engine.operation_request().expect("問いが立っていない");

    // 型として持っていません。**入れる場所がありません。**
    assert_eq!(asked.runs, "systemctl reload httpd");
}

#[tokio::test]
async fn a_secret_the_person_typed_is_spent_with_the_approval() {
    // **1 回の許可で 1 回だけ**（D48）。使ったら消えます。
    // 残ると、**人が見ていない間に何度でも上がれます。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    engine
        .answer_operation(Actor::Human, true, Some("not-a-real-password".into()))
        .await
        .expect("人が許せない");

    // 1 回目は許可を使う（繋がっていないので、そこで止まるのが正しい）。
    let used = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(used, Err(EngineError::NotConnected)),
        "許可が使われていない: {used:?}"
    );

    // 2 回目は、また尋ねる。**秘密も一緒に消えています。**
    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "**秘密が残っている**: {again:?}"
    );
}

#[tokio::test]
async fn refusing_throws_the_secret_away_too() {
    // 断ったのに秘密だけ残る、を作らない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    engine
        .answer_operation(Actor::Human, false, Some("not-a-real-password".into()))
        .await
        .expect("人が断れない");

    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "断ったのに走ろうとしている: {again:?}"
    );
}

#[tokio::test]
async fn the_ai_cannot_hand_itself_a_secret() {
    // **自分で自分に権限を渡せたら、承認の意味がありません。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let refused = engine
        .answer_operation(Actor::Ai, true, Some("not-a-real-password".into()))
        .await;

    assert!(refused.is_err(), "AI が自分へ秘密を渡せる: {refused:?}");
    assert!(engine.operation_request().is_some(), "問いが消えている");
}

#[tokio::test]
async fn the_ai_cannot_answer_its_own_request() {
    // **自分で自分を許可できたら、承認の意味がありません。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    let refused = engine.answer_operation(Actor::Ai, true, None).await;

    assert!(refused.is_err(), "AI が自分で許可できてしまう: {refused:?}");
    assert!(engine.operation_request().is_some(), "問いが消えている");
}

#[tokio::test]
async fn refusing_leaves_the_operation_unrunnable() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;

    engine
        .answer_operation(Actor::Human, false, None)
        .await
        .expect("人が断れない");

    assert!(engine.operation_request().is_none(), "問いが残っている");
    // 断ったのだから、次に呼んでも走らない（また尋ねる）。
    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "断ったのに走ろうとしている: {again:?}"
    );
}

#[tokio::test]
async fn an_approval_is_spent_once() {
    // **使い切り。**1 回許したら 1 回だけ。
    // 許しっぱなしにすると、**人が見ていない間に何度でも走ります。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");

    // 1 回目は許可を使う（繋がっていないので、そこで止まるのが正しい）。
    let used = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(used, Err(EngineError::NotConnected)),
        "許可が使われていない: {used:?}"
    );

    // 2 回目は、また尋ねる。
    let again = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(
        matches!(again, Err(EngineError::ConsoleApprovalNeeded)),
        "**1 回の許可で 2 回走ろうとしている**: {again:?}"
    );
}

#[tokio::test]
async fn a_person_never_has_to_ask_anyone() {
    // **人は承認も上限も要りません。**画面の前に居るのだから、見えています。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);

    let ran = engine.run_operation(Actor::Human, "reload-httpd").await;

    // 繋がっていないので、そこで止まるのが正しい。**承認では止まらない。**
    assert!(
        matches!(ran, Err(EngineError::NotConnected)),
        "人が承認を求められている: {ran:?}"
    );
    assert!(engine.operation_request().is_none(), "人に問いが立っている");
}

#[tokio::test]
async fn the_hourly_ceiling_actually_stops_it() {
    // **README にも承認の画面にも「1 時間あたりの上限」があると書いてあります。**
    // `max_per_hour = 0` は書けない、という検証まで入れてあります。
    //
    // **効いていませんでした。**
    //
    // 数えていた入れ物が、**許可の札と同じもの**でした。札は走るときに
    // 取り出されて消えるので、**走った回数は 1 件も残りません。**
    // `lately` は多くても 1 で、`max_per_hour` は 1 以上と決めてあるため、
    // **`lately > max_per_hour` は決して真になりません。**
    //
    // 製品が「止まる所がある」と言っておいて、止まらないのが一番悪い形です。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"once-an-hour\"\n\
         run = \"systemctl reload httpd\"\n\
         description = \"1 時間に 1 回だけ\"\nmax_per_hour = 1\n",
    )
    .expect("一覧を書けない");
    let engine = engine_in(&dir);

    // 1 回目。**人が許して、走る**（繋がっていないのでそこで止まるのが正しい）。
    let _ = engine.run_operation(Actor::Ai, "once-an-hour").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");
    let first = engine.run_operation(Actor::Ai, "once-an-hour").await;
    assert!(
        matches!(first, Err(EngineError::NotConnected)),
        "1 回目が走っていない: {first:?}"
    );

    // 2 回目。**人がもう一度許しても、上限で止まる。**
    let _ = engine.run_operation(Actor::Ai, "once-an-hour").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");
    let second = engine.run_operation(Actor::Ai, "once-an-hour").await;

    assert!(
        matches!(second, Err(EngineError::OperationCeiling { .. })),
        "**1 時間あたりの上限が効いていない**: {second:?}"
    );
}

#[tokio::test]
async fn a_person_is_not_stopped_by_the_ceiling() {
    // **上限は AI にかかります。**画面の前に居る人は、見えているので止めません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"once-an-hour\"\n\
         run = \"systemctl reload httpd\"\n\
         description = \"1 時間に 1 回だけ\"\nmax_per_hour = 1\n",
    )
    .expect("一覧を書けない");
    let engine = engine_in(&dir);

    for round in 1..=3 {
        let ran = engine.run_operation(Actor::Human, "once-an-hour").await;
        assert!(
            matches!(ran, Err(EngineError::NotConnected)),
            "{round} 回目で人が止められている: {ran:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn an_approval_that_is_never_used_does_not_keep_the_secret_forever() {
    // **人が［許可］を押したのに、AI が呼び返してこないことは在ります**
    // （会話が途切れる・別の話に移る）。そのとき**パスワードを抱えた札が
    // 残り続ける**なら、「保存しない」と言っている意味が薄れます。
    //
    // 時計を進めて確かめます（`tokio::time` の時計）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;
    engine
        .answer_operation(Actor::Human, true, Some("not-a-real-password".into()))
        .await
        .expect("人が許せない");

    // **6 分後。**押されたことを忘れているべき時間です。
    tokio::time::advance(std::time::Duration::from_secs(6 * 60)).await;

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(refused, Err(EngineError::ConsoleApprovalNeeded)),
        "**古い許可がそのまま使えている**: {refused:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn an_approval_still_works_a_moment_later() {
    // **短くしすぎない。**AI が呼び返すのは普通は数秒後です。
    // ここが厳しすぎると、**押しても走らない**という別の壊れ方になります。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir);
    let engine = engine_in(&dir);
    let _ = engine.run_operation(Actor::Ai, "reload-httpd").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");

    tokio::time::advance(std::time::Duration::from_secs(30)).await;

    let used = engine.run_operation(Actor::Ai, "reload-httpd").await;

    assert!(
        matches!(used, Err(EngineError::NotConnected)),
        "30 秒で許可が切れている: {used:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn the_ceiling_lets_go_after_an_hour() {
    // **上限は「1 時間あたり」です。**永久に止めるものではありません。
    // 明けないなら、それは上限ではなく禁止です。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(
        dir.path().join("operations.toml"),
        "version = 1\n\n[[operation]]\nid = \"once-an-hour\"\n\
         run = \"systemctl reload httpd\"\n\
         description = \"1 時間に 1 回だけ\"\nmax_per_hour = 1\n",
    )
    .expect("一覧を書けない");
    let engine = engine_in(&dir);

    let _ = engine.run_operation(Actor::Ai, "once-an-hour").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");
    let _ = engine.run_operation(Actor::Ai, "once-an-hour").await;

    // **1 時間と 1 分後。**窓から出ているべき時間です。
    tokio::time::advance(std::time::Duration::from_secs(61 * 60)).await;

    let _ = engine.run_operation(Actor::Ai, "once-an-hour").await;
    engine
        .answer_operation(Actor::Human, true, None)
        .await
        .expect("人が許せない");
    let again = engine.run_operation(Actor::Ai, "once-an-hour").await;

    assert!(
        matches!(again, Err(EngineError::NotConnected)),
        "**1 時間経っても明けていない**: {again:?}"
    );
}

// --- 操作の断り文が、読み取りの許可リストを案内していた（2026-09-27） ----------
//
// `run_operation` は `refuse_readonly` をそのまま呼んでいました。つまり ——
//
// ```
// 知らない操作 id → 「**readonly.toml** に書いてください」
// 実際の書く先   → **operations.toml**
// ```
//
// **正しい所へ書いても直らないほうへ、人を案内していました。**
// 断った記録も `readonly-refused.log` へ落ちていたので、
// **どちらのファイルが足りなかったのかが、記録からも分かりません。**

#[tokio::test]
async fn an_unknown_operation_points_at_operations_toml_not_the_readonly_list() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    allow(&dir); // reload-httpd だけ在る
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "restart-everything").await;
    assert!(matches!(
        refused,
        Err(EngineError::OperationNotAllowed { .. })
    ));

    let said = engine
        .diagnostics()
        .recent(20)
        .into_iter()
        .map(|event| format!("{} {}", event.message, event.hint.unwrap_or_default()))
        .collect::<Vec<_>>();

    assert!(
        said.iter().any(|line| line.contains("operations.toml")),
        "**書く先を案内していない**: {said:?}"
    );
    assert!(
        !said.iter().any(|line| line.contains("readonly.toml")),
        "**違うファイルを案内している**: {said:?}"
    );
    // **在るものを見せる。**打ち間違いなら、これで気づけます。
    assert!(
        said.iter().any(|line| line.contains("reload-httpd")),
        "**在る id を見せていない**: {said:?}"
    );
}

#[tokio::test]
async fn an_empty_operations_file_says_so() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    // **1 行も書かない。**
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;
    assert!(matches!(
        refused,
        Err(EngineError::OperationNotAllowed { .. })
    ));

    let said = engine
        .diagnostics()
        .recent(20)
        .into_iter()
        .map(|event| format!("{} {}", event.message, event.hint.unwrap_or_default()))
        .collect::<Vec<_>>();

    assert!(
        said.iter()
            .any(|line| line.contains("まだ 1 本も書かれていません")),
        "**空であることを言っていない**: {said:?}"
    );
    assert!(
        said.iter().any(|line| line.contains("operations.toml")),
        "**どこへ書けばいいかを言っていない**: {said:?}"
    );
}

/// **断りの文が、正しいファイルを指すこと**（実運用の報告・2026-10-23）。
///
/// 本番で起きたこと ——
///
/// > 人の受け取り方は「sshboard に /etc を書き換える操作が無い」だった。
/// > 「空から始めるのが仕様」という前提は、人には伝わっていなかった。
/// > 結果として、操作は sshboard の画面の外で行われた。
/// > **共有画面に残らず、記録もされない経路に流れた。**
///
/// 原因のひとつがここでした —— 操作を断ったとき、
/// AI が受け取る文は **`readonly.toml`** を指していました。
/// 操作の置き場は **`operations.toml`** です。
/// `NotAllowed` を読み取り専用コマンドと共用していたためです。
///
/// **AI はその間違ったファイル名を人へ伝えます。**
/// 人は書く場所を探して見つけられず、画面の外へ出ました ——
/// `run_command` を作らない決定（D3）が守ろうとしたもの
/// 「**人の見ていない所で終わらせない**」が、まさに裏返った形です。
#[tokio::test]
async fn the_refusal_names_the_file_the_person_must_write() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;
    let said = format!("{}", refused.expect_err("通ってしまった"));

    assert!(
        said.contains("operations.toml"),
        "書く場所を指していない: {said}"
    );
    assert!(
        !said.contains("readonly.toml"),
        "**違うファイルを指している**: {said}"
    );
}

/// **既定が空であることを、断りの文で言うこと。**
///
/// 「無い」と「まだ書いていない」は別です。
/// 前者だと読まれると、人は**製品に機能が無い**と受け取ります
/// （実際にそう受け取られました）。
#[tokio::test]
async fn the_refusal_says_the_list_starts_empty() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine.run_operation(Actor::Ai, "reload-httpd").await;
    let said = format!("{}", refused.expect_err("通ってしまった"));

    assert!(
        said.contains("既定") || said.contains("まだ"),
        "「まだ書いていない」と言っていない: {said}"
    );
}

/// **断ったことが帯に出ること。**
///
/// それまで断りは**診断にしか出ていませんでした** —— 人が開かないタブです。
/// 帯は端末の横に常設されたので（UI 案・3 列）、**そこに出れば人は見ます。**
#[tokio::test]
async fn the_refusal_shows_up_in_the_band() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let path = dir.path().join("connections.toml");
    std::fs::write(&path, "version = 1\n").expect("接続一覧を書けない");

    let band = Band::new();
    let mut subscriber = band.subscribe();
    let engine = Engine::new(band, Arc::new(OutputStream::new()), path);

    tokio::spawn(async move { engine.run_operation(Actor::Ai, "reload-httpd").await });

    let event = subscriber.recv().await.expect("帯へ出ていない");
    event.ack();

    assert!(
        event.line().text().contains("reload-httpd"),
        "帯に出ていない: {}",
        event.line().text()
    );
    assert!(
        event.line().text().contains("operations.toml"),
        "**帯でも書く場所を言っていない**: {}",
        event.line().text()
    );
}

// --- AI の提案と、人の登録（実運用の報告・2026-10-23）---------------------
//
// **これは `run_command` ではありません。**
// AI が出すのは提案（ただのデータ）で、**書くのは人**です。
// そして**登録と実行は別の行為**です。

/// **AI の提案が、踏ませない語を含むなら、人に見せる前に断る。**
///
/// 見せてから断ると、人は「押したのに入らない」に当たります。
#[tokio::test]
async fn a_proposal_with_a_forbidden_word_never_reaches_the_person() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine
        .propose_operation(
            Actor::Ai,
            sshboard_engine::OperationProposal {
                id: "sneaky".into(),
                run: "sudo cat /root/.ssh/id_ed25519".into(),
                description: "鍵を読む".into(),
                max_per_hour: 1,
            },
        )
        .await;

    assert!(refused.is_err(), "踏ませない語が通っている");
    assert!(
        engine.operation_proposal().is_none(),
        "**人に見せる所まで届いている**"
    );
}

/// **上限の無い提案は入れられない。**止まる所が要ります。
#[tokio::test]
async fn a_proposal_without_a_ceiling_is_refused() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    let refused = engine
        .propose_operation(
            Actor::Ai,
            sshboard_engine::OperationProposal {
                id: "no-ceiling".into(),
                run: "sudo systemctl reload httpd".into(),
                description: "読み直させる".into(),
                max_per_hour: 0,
            },
        )
        .await;

    assert!(refused.is_err(), "上限 0 が通っている");
}

/// **提案を出しても、まだ走らない。**
///
/// ここが `run_command` との違いです。**提案は実行権ではありません。**
#[tokio::test]
async fn proposing_does_not_make_it_runnable() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);

    engine
        .propose_operation(
            Actor::Ai,
            sshboard_engine::OperationProposal {
                id: "httpd-reload".into(),
                run: "sudo systemctl reload httpd".into(),
                description: "読み直させる".into(),
                max_per_hour: 2,
            },
        )
        .await
        .expect("提案が通らない");

    let refused = engine.run_operation(Actor::Ai, "httpd-reload").await;
    assert!(
        matches!(refused, Err(EngineError::OperationNotAllowed { .. })),
        "**提案しただけで走った**: {refused:?}"
    );
}

/// **人が登録すると、一覧に入る。**書くのは人の側の口です。
#[tokio::test]
async fn the_person_registering_it_puts_it_on_the_list() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);
    let proposal = sshboard_engine::OperationProposal {
        id: "httpd-reload".into(),
        run: "sudo systemctl reload httpd".into(),
        description: "読み直させる".into(),
        max_per_hour: 2,
    };

    engine
        .propose_operation(Actor::Ai, proposal.clone())
        .await
        .expect("提案が通らない");
    engine.register_operation(&proposal).expect("登録できない");

    let listed = engine.operations().expect("読めない");
    assert!(listed.get("httpd-reload").is_some(), "一覧に入っていない");
    // **出した提案は下がる。**出したままだと二度登録されます。
    assert!(engine.operation_proposal().is_none(), "提案が残っている");
}

/// **登録しても、AI はまだ走らせられない。**承認の問いを通ります（D42）。
#[tokio::test]
async fn registering_is_not_the_same_as_allowing_it_to_run() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);
    let proposal = sshboard_engine::OperationProposal {
        id: "httpd-reload".into(),
        run: "sudo systemctl reload httpd".into(),
        description: "読み直させる".into(),
        max_per_hour: 2,
    };
    engine.register_operation(&proposal).expect("登録できない");

    // 繋いでいないので走りませんが、**断られ方が「承認が無い」側**であること。
    let asked = engine.run_operation(Actor::Ai, "httpd-reload").await;
    assert!(
        !matches!(asked, Err(EngineError::OperationNotAllowed { .. })),
        "登録したのに一覧に無い扱い: {asked:?}"
    );
    assert!(
        engine.operation_request().is_some(),
        "**承認の問いが立っていない** —— 登録＝実行になっています"
    );
}

/// **同じ id を 2 つにしない。**どちらが走るのか人が読めません。
#[tokio::test]
async fn registering_the_same_id_twice_is_refused() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);
    let proposal = sshboard_engine::OperationProposal {
        id: "httpd-reload".into(),
        run: "sudo systemctl reload httpd".into(),
        description: "読み直させる".into(),
        max_per_hour: 2,
    };
    engine.register_operation(&proposal).expect("1 回目");

    let again = engine.register_operation(&proposal);
    assert!(again.is_err(), "**上書きしている**");
}

/// **断ったら何も書かない。**
#[tokio::test]
async fn dismissing_a_proposal_writes_nothing() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_in(&dir);
    engine
        .propose_operation(
            Actor::Ai,
            sshboard_engine::OperationProposal {
                id: "httpd-reload".into(),
                run: "sudo systemctl reload httpd".into(),
                description: "読み直させる".into(),
                max_per_hour: 2,
            },
        )
        .await
        .expect("提案が通らない");

    engine.dismiss_proposal();

    assert!(engine.operation_proposal().is_none(), "提案が残っている");
    assert!(
        !dir.path().join("operations.toml").exists(),
        "**断ったのにファイルが出来ている**"
    );
}
