//! sshboard のデスクトップアプリ。
//!
//! **GUI と MCP が同じプロセスに同居する**（decisions D8 / D15）。
//! 裏で見えないセッションを 1 本も増やさないための前提（PRD §4-1）。

mod bridge;
mod commands;
mod mcp_host;
mod pending;

use sshboard_band::Band;
use tauri::Manager;

use crate::commands::McpUrl;
use crate::pending::PendingLines;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::band_ack,
            commands::mcp_url
        ])
        .setup(|app| {
            let band = Band::new();

            // **購読は MCP を立てる前に済ませる。**
            // 逆にすると、画面が繋がる前に来た呼び出しが帯へ出ないまま通ってしまう。
            let subscriber = band.subscribe();

            app.manage(PendingLines::new());
            app.manage(McpUrl::default());
            app.manage(band.clone());

            bridge::spawn(app.handle().clone(), subscriber);
            mcp_host::spawn(app.handle().clone(), band);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri アプリを起動できません");
}
