//! 画面から呼ばれる口。

use std::sync::{Arc, Mutex};

use tauri::State;

use sshboard_band::Actor;
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;

use crate::pending::PendingLines;

/// MCP クライアントへ渡すもの一式。
///
/// **合言葉はこの起動の間だけ有効**（D23）。ファイルには書きません。
#[derive(Clone, serde::Serialize)]
pub struct McpAccess {
    pub url: String,
    pub token: String,
}

/// MCP の口。立ち上がるまでは `None`。
#[derive(Default)]
pub struct McpUrl(Mutex<Option<McpAccess>>);

impl McpUrl {
    pub fn set(&self, url: String, token: String) {
        match self.0.lock() {
            Ok(mut held) => *held = Some(McpAccess { url, token }),
            Err(_) => eprintln!("[sshboard] MCP の口を保持できません"),
        }
    }

    pub fn get(&self) -> Option<McpAccess> {
        self.0.lock().ok().and_then(|held| held.clone())
    }
}

/// 画面が「この行を描いた」と返す。
#[tauri::command]
pub fn band_ack(seq: u64, pending: State<'_, PendingLines>) -> Result<(), String> {
    pending.release(seq).map_err(|error| error.to_string())
}

/// MCP クライアントへ登録する URL と合言葉を画面に見せるため。
#[tauri::command]
pub fn mcp_url(url: State<'_, McpUrl>) -> Option<McpAccess> {
    url.get()
}

/// サーバーのログを追う（`tail -f`）。**GUI へは色付き・MCP へは素で流れます**（Issue 005）。
///
/// 引数は**パスだけ**です。コマンドはこちらで組み立てるので、
/// **任意の文字列がシェルへ渡ることはありません**（D3）。
#[tauri::command]
pub async fn stream_follow(
    path: String,
    lines: Option<u32>,
    engine: State<'_, Arc<Engine>>,
    stream: State<'_, Arc<OutputStream>>,
) -> Result<(), String> {
    // **止めたあとでも、人が押したら流す**（PRD §4-3「止めた後、人が同じセッションで
    // 続きをやれる」）。止めたら二度と流せない、では「止められる」ではなく「壊れる」。
    stream.resume();

    let engine = Arc::clone(&engine);
    let lines = lines.unwrap_or(DEFAULT_TAIL_LINES).clamp(1, 5000);
    // 追い続けるので、ここでは待たない。**失敗したら記録に残る**（診断タブ）。
    tauri::async_runtime::spawn(async move {
        if let Err(error) = engine.follow(Actor::Human, &path, lines).await {
            eprintln!("[sshboard] 追えません: {error}");
        }
    });
    Ok(())
}

/// 最初に何行さかのぼるか。**多すぎると画面が埋まる。**
const DEFAULT_TAIL_LINES: u32 = 200;

/// 人が止める（PRD §4-3）。**止めたあとは流れない。**
#[tauri::command]
pub fn stop_stream(stream: State<'_, Arc<OutputStream>>) {
    stream.stop();
}
