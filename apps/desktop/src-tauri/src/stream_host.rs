//! 追尾している出力を画面へ流す（Issue 005）。
//!
//! **MCP と同じ 1 本を共有する。**GUI 用にもう 1 本流すのは
//! 「裏で見えないセッションを張らない」（PRD §4-1）に反する。

use std::sync::Arc;

use sshboard_stream::{OutputStream, RecvError};
use tauri::{AppHandle, Emitter};

/// 画面が待ち受けるイベント名。**ANSI を落とさずに渡す。**
pub const STREAM_EVENT: &str = "stream://raw";

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
