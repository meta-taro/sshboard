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

/// 実際のログに出てくる形を並べる。
/// 色・タイトル変更（OSC）・CRLF・日本語。**どれも境界の踏み方が違う。**
fn demo_lines() -> Vec<&'static [u8]> {
    vec![
        b"\x1b]0;sshboard demo\x07",
        b"\x1b[2m2026-08-26 19:40:01\x1b[0m \x1b[32mINFO\x1b[0m  service started\r\n",
        b"\x1b[2m2026-08-26 19:40:02\x1b[0m \x1b[32mINFO\x1b[0m  listening\r\n",
        b"\x1b[2m2026-08-26 19:40:05\x1b[0m \x1b[33mWARN\x1b[0m  queue is filling up\r\n",
        "\x1b[2m2026-08-26 19:40:07\x1b[0m \x1b[36mINFO\x1b[0m  設定ファイルを読み込みました\r\n"
            .as_bytes(),
        b"\x1b[2m2026-08-26 19:40:09\x1b[0m \x1b[31mERROR\x1b[0m disk usage 92%\r\n",
        b"\x1b[2m2026-08-26 19:40:11\x1b[0m \x1b[31mERROR\x1b[0m \x1b[1mno space left\x1b[0m\r\n",
    ]
}
