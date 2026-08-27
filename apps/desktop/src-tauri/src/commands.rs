//! 画面から呼ばれる口。

use std::sync::{Arc, Mutex};

use tauri::State;

use sshboard_stream::OutputStream;

use crate::pending::PendingLines;
use crate::stream_host;

/// MCP の口の URL。立ち上がるまでは `None`。
#[derive(Default)]
pub struct McpUrl(Mutex<Option<String>>);

impl McpUrl {
    pub fn set(&self, url: String) {
        match self.0.lock() {
            Ok(mut held) => *held = Some(url),
            Err(_) => eprintln!("[sshboard] MCP の URL を保持できません"),
        }
    }

    pub fn get(&self) -> Option<String> {
        self.0.lock().ok().and_then(|held| held.clone())
    }
}

/// 画面が「この行を描いた」と返す。
#[tauri::command]
pub fn band_ack(seq: u64, pending: State<'_, PendingLines>) -> Result<(), String> {
    pending.release(seq).map_err(|error| error.to_string())
}

/// MCP クライアントへ登録する URL を画面に見せるため。
#[tauri::command]
pub fn mcp_url(url: State<'_, McpUrl>) -> Option<String> {
    url.get()
}

/// **Phase 0 限りの確認用。**サーバーへ繋がずに色付きの出力を流す。
/// 002 が通ったら本物の `tail -f` に差し替える。
#[tauri::command]
pub fn start_demo_stream(stream: State<'_, Arc<OutputStream>>) -> Result<(), String> {
    // **止めたあとでも、人が押したら流す**（PRD §4-3「止めた後、人が同じセッションで
    // 続きをやれる」）。止めたら二度と流せない、では「止められる」ではなく「壊れる」。
    stream.resume();
    stream_host::spawn_demo(Arc::clone(&stream));
    Ok(())
}

/// 人が止める（PRD §4-3）。**止めたあとは流れない。**
#[tauri::command]
pub fn stop_stream(stream: State<'_, Arc<OutputStream>>) {
    stream.stop();
}
