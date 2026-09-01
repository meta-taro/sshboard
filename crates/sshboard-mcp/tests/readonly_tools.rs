//! `run_readonly` の MCP 側（D3）。**サーバーを一切使いません。**
//!
//! ここで見るのは 2 つだけです。
//!
//! 1. **何が許されているかを AI が自分で引ける**（引けないと、当てずっぽうで呼ぶ）
//! 2. **許されていないものは、断り方まで含めて AI に伝わる**
//!    「駄目でした」で終わると、AI は人へ何を頼めばよいか分からない

use std::sync::Arc;

use sshboard_band::Band;
use sshboard_engine::Engine;
use sshboard_mcp::SshboardMcp;
use sshboard_stream::OutputStream;

fn server_in(dir: &tempfile::TempDir) -> SshboardMcp {
    let connections = dir.path().join("connections.toml");
    std::fs::write(&connections, "version = 1\n").expect("接続一覧を書けない");

    let band = Band::new();
    let stream = Arc::new(OutputStream::new());
    let engine = Engine::new(band.clone(), Arc::clone(&stream), connections);
    SshboardMcp::new(band, stream).with_engine(Arc::new(engine))
}

#[tokio::test]
async fn the_list_is_empty_until_a_human_puts_something_in_it() {
    // **既定は空**（D3 追記）。ここに製品が用意した既定が混ざっていたら、
    // それは誰かが「たぶん要るだろう」で書いたということ。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let server = server_in(&dir);

    let answer = server
        .list_readonly_commands()
        .await
        .expect("一覧は空でも読める");

    let parsed: serde_json::Value = serde_json::from_str(&answer).expect("JSON で返る");
    assert_eq!(parsed["commands"].as_array().expect("配列").len(), 0);
}

#[tokio::test]
async fn the_list_shows_what_the_human_wrote_including_what_actually_runs() {
    // **何が走るかを隠さない。**中身の分からないものを呼ばせる方が危ない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    std::fs::write(
        dir.path().join("readonly.toml"),
        "version = 1\n\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\ndescription = \"稼働時間\"\n",
    )
    .expect("許可リストを書けない");
    let server = server_in(&dir);

    let answer = server.list_readonly_commands().await.expect("読める");

    let parsed: serde_json::Value = serde_json::from_str(&answer).expect("JSON で返る");
    let first = &parsed["commands"][0];
    assert_eq!(first["id"], "uptime");
    assert_eq!(first["run"], "uptime");
    assert_eq!(first["description"], "稼働時間");
}

#[tokio::test]
async fn an_unlisted_id_comes_back_as_something_the_ai_can_act_on() {
    // **AI が人へ何を頼めばよいかが分かる文面で返す**（product-baseline §17）。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let server = server_in(&dir);

    let error = server
        .run_readonly(rmcp::handler::server::wrapper::Parameters(
            sshboard_mcp::ReadonlyCommandId {
                command_id: "systemctl-restart-nginx".to_string(),
            },
        ))
        .await
        .expect_err("許可していないものが通った");

    let message = error.message.to_string();
    assert!(
        message.contains("systemctl-restart-nginx"),
        "何を断ったのか分からない: {message}"
    );
    assert!(
        message.contains("readonly.toml"),
        "どこに足せばよいか分からない: {message}"
    );
}
