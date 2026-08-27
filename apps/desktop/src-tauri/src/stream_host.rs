//! 追尾している出力を画面へ流す（Issue 005）。
//!
//! **MCP と同じ 1 本を共有する。**GUI 用にもう 1 本流すのは
//! 「裏で見えないセッションを張らない」（PRD §4-1）に反する。

use std::sync::Arc;
use std::time::Duration;

use sshboard_stream::{OutputStream, RecvError};
use tauri::{AppHandle, Emitter};

/// 画面が待ち受けるイベント名。**ANSI を落とさずに渡す。**
pub const STREAM_EVENT: &str = "stream://raw";

/// Phase 0 の確認用に流す行の間隔。
const DEMO_INTERVAL: Duration = Duration::from_millis(220);

/// 生の出力を画面へ流し続ける。
pub fn spawn_bridge(app: AppHandle, stream: Arc<OutputStream>) {
    let mut raw = stream.subscribe_raw();

    tauri::async_runtime::spawn(async move {
        loop {
            match raw.recv().await {
                Ok(chunk) => {
                    if let Err(error) = app.emit(STREAM_EVENT, chunk) {
                        eprintln!("[sshboard] 出力を画面へ渡せません: {error}");
                    }
                }
                // 取りこぼしたことを黙らない。
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("[sshboard] 出力を {missed} 個取りこぼしました");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// **Phase 0 限りの確認用。**サーバーへ繋がずに、色付きの出力を流す。
///
/// 005 の完了条件のうち「GUI は色付き / MCP はプレーン」を、
/// 実機を待たずに確かめるためのもの。**002 が通ったら本物の `tail -f` に差し替える。**
pub fn spawn_demo(stream: Arc<OutputStream>) {
    tauri::async_runtime::spawn(async move {
        for line in demo_lines() {
            if stream.push(line).is_err() {
                break; // 人が止めた
            }
            tokio::time::sleep(DEMO_INTERVAL).await;
        }
    });
}

/// **合成の出力。サーバーへは 1 台も繋いでいない。**
///
/// 本物のログに見える偽データを画面へ出さないこと。**見た人が本物と誤解する。**
/// ここで確かめたいのは中身ではなく、色・OSC・CRLF・日本語という
/// **境界の踏み方が違う並びが、面ごとに正しく分かれるか**だけ。
fn demo_lines() -> Vec<&'static [u8]> {
    vec![
        b"\x1b]0;sshboard demo\x07",
        "\x1b[1;33m--- ここから下は合成の出力です。サーバーへは繋いでいません ---\x1b[0m\r\n"
            .as_bytes(),
        b"\x1b[2mDEMO\x1b[0m \x1b[32mINFO\x1b[0m  \x1b[2m(fake)\x1b[0m green text\r\n",
        b"\x1b[2mDEMO\x1b[0m \x1b[33mWARN\x1b[0m  \x1b[2m(fake)\x1b[0m yellow text\r\n",
        b"\x1b[2mDEMO\x1b[0m \x1b[31mERROR\x1b[0m \x1b[2m(fake)\x1b[0m red text\r\n",
        b"\x1b[2mDEMO\x1b[0m \x1b[31mERROR\x1b[0m \x1b[1mbold red text\x1b[0m\r\n",
        "\x1b[2mDEMO\x1b[0m \x1b[36mINFO\x1b[0m  \x1b[2m(fake)\x1b[0m 日本語の幅を見るための行です\r\n"
            .as_bytes(),
        "\x1b[1;33m--- ここまで。実機のログは 002 が通ってから流します ---\x1b[0m\r\n"
            .as_bytes(),
    ]
}
