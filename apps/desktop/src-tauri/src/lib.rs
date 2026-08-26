//! sshboard のデスクトップアプリ。
//!
//! **GUI と MCP が同じプロセスに同居する**（decisions D8 / D15）。
//! 裏で見えないセッションを 1 本も増やさないための前提（PRD §4-1）。

mod bridge;
mod commands;
mod mcp_host;
mod pending;
mod stream_host;

use std::sync::Arc;

use sshboard_band::Band;
use sshboard_stream::OutputStream;
use tauri::Manager;

use crate::commands::McpUrl;
use crate::pending::PendingLines;

/// 立ち上げと同時に Phase 0 の確認用の出力を流す環境変数。
/// **人が押すボタンと同じことを、自動で確かめるためだけのもの。**
/// 002 が通って本物の `tail -f` に差し替えたら消す。
const PHASE0_DEMO_ENV: &str = "SSHBOARD_PHASE0_DEMO";

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::band_ack,
            commands::mcp_url,
            commands::start_demo_stream,
            commands::stop_stream
        ])
        .setup(|app| {
            let band = Band::new();

            // **購読は MCP を立てる前に済ませる。**
            // 逆にすると、画面が繋がる前に来た呼び出しが帯へ出ないまま通ってしまう。
            let subscriber = band.subscribe();

            app.manage(PendingLines::new());
            app.manage(McpUrl::default());
            app.manage(band.clone());

            // 追尾している出力。**GUI と MCP で同じ 1 本を共有する**（PRD §4-1）。
            let stream = Arc::new(OutputStream::new());
            app.manage(Arc::clone(&stream));

            bridge::spawn(app.handle().clone(), subscriber);
            stream_host::spawn_bridge(app.handle().clone(), Arc::clone(&stream));

            // **Phase 0 限り。**画面のボタンは人が押すものなので、
            // 自動で確かめたいときのために口を 1 つ開けておく。
            // 002 が通って本物の `tail -f` に差し替えたら消す。
            if std::env::var_os(PHASE0_DEMO_ENV).is_some() {
                stream_host::spawn_demo(Arc::clone(&stream));
            }
            mcp_host::spawn(app.handle().clone(), band, stream);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri アプリを起動できません");
}
