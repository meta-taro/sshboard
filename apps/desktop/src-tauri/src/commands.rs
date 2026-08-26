//! 画面から呼ばれる口。

use std::sync::Mutex;

use tauri::State;

use crate::pending::PendingLines;

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
