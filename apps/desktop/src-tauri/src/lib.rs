//! sshboard のデスクトップアプリ。
//!
//! **GUI と MCP が同じプロセスに同居する**（decisions D8 / D15）。
//! 裏で見えないセッションを 1 本も増やさないための前提（PRD §4-1）。

mod bridge;
mod capture;
mod commands;
mod connections_cmd;
mod mcp_host;
mod menu;
mod pending;
mod session_cmd;
mod stream_host;
mod version;

use std::sync::Arc;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;
use tauri::Manager;

use crate::commands::McpUrl;
use crate::pending::PendingLines;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // 自動更新（D34）。**黙って入れ替えません。**
        // 見つけたら画面に出し、押すのは人。SSH の鍵を扱う道具が
        // 無断で自分を書き換えるのは、この製品の性格に合いません。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // **押しても何も起きない項目を残さない。**受け手はここ 1 か所。
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            commands::band_ack,
            commands::mcp_url,
            commands::stream_follow,
            commands::stop_stream,
            connections_cmd::connections_list,
            connections_cmd::connections_path,
            connections_cmd::connection_save,
            connections_cmd::connection_delete,
            connections_cmd::inspect_key_file,
            session_cmd::session_connect,
            session_cmd::session_disconnect,
            session_cmd::session_status,
            session_cmd::session_focus,
            session_cmd::remote_list_dir,
            session_cmd::remote_read_file,
            session_cmd::remote_ensure_dir,
            session_cmd::remote_upload,
            session_cmd::remote_download,
            session_cmd::local_list_dir,
            session_cmd::console_open,
            session_cmd::console_type,
            session_cmd::console_resize,
            session_cmd::console_take,
            session_cmd::console_stop,
            session_cmd::console_holder,
            session_cmd::diagnostics_recent,
            menu::set_menu_labels
        ])
        .setup(|app| {
            let band = Band::new();

            // **購読は MCP を立てる前に済ませる。**
            // 逆にすると、画面が繋がる前に来た呼び出しが帯へ出ないまま通ってしまう。
            let subscriber = band.subscribe();

            // 接続一覧が変わったことを配る口。**人と AI の両方が書き換える**ので、
            // 画面へ押し出さないと、AI が足した接続を人が知らないままになる（PRD §4-2）。
            let connections_watch = Arc::new(ConnectionsWatch::new());
            app.manage(Arc::clone(&connections_watch));
            connections_cmd::spawn_bridge(app.handle().clone(), Arc::clone(&connections_watch));

            // 接続一覧の置き場所。**OS の既定の場所**（Windows / macOS で別）。
            let connections_path = match sshboard_connections::default_path() {
                Ok(path) => {
                    app.manage(connections_cmd::ConnectionsPath(path.clone()));
                    Some(path)
                }
                // 置き場所が分からないなら、接続管理だけが使えない。
                // **アプリ全体を止めない。**
                Err(error) => {
                    eprintln!("[sshboard] 接続一覧の置き場所が分かりません: {error}");
                    None
                }
            };

            app.manage(PendingLines::new());
            app.manage(McpUrl::default());
            app.manage(band.clone());

            // 追尾している出力。**GUI と MCP で同じ 1 本を共有する**（PRD §4-1）。
            let stream = Arc::new(OutputStream::new());
            app.manage(Arc::clone(&stream));

            bridge::spawn(app.handle().clone(), subscriber);
            stream_host::spawn_bridge(app.handle().clone(), Arc::clone(&stream));

            // **すべての操作が通る 1 か所**（PRD §4-1）。
            // GUI も MCP もここを共有する。ここを迂回した経路を作った瞬間に
            // 「裏で見えない SSH」が生まれる。
            let engine = Arc::new(Engine::with_diagnostics(
                band.clone(),
                Arc::clone(&stream),
                // 置き場所が分からないときは、存在しないパスを渡して
                // **「登録されていません」と正直に断らせる。**繋げてしまうよりよい。
                connections_path.unwrap_or_else(|| std::path::PathBuf::from("connections.toml")),
                sshboard_diag::Diagnostics::new(),
            ));
            app.manage(Arc::clone(&engine));
            session_cmd::spawn_bridge(app.handle().clone(), Arc::clone(&engine));
            // **AI が握った瞬間に、人の側の入力が締まる**必要がある（D29）。
            session_cmd::spawn_console_bridge(app.handle().clone(), Arc::clone(&engine));

            mcp_host::spawn(
                app.handle().clone(),
                band,
                stream,
                connections_watch,
                engine,
            );

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri アプリを起動できません");
}
