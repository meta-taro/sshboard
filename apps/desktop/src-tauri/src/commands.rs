//! 画面から呼ばれる口。

use std::sync::{Arc, Mutex};

use tauri::State;

use sshboard_band::Actor;
use sshboard_engine::Engine;
use sshboard_stream::OutputStream;

use crate::connections_cmd::ConnectionsPath;
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
/// **MCP の口を閉じる**（UI 案の「AI を止める」・2026-10-23）。
///
/// それまで「AI を止める」は、**走っているもの**（流れている出力と、AI が
/// 握っている端末）しか止められませんでした。**口はそのまま空いていた**ので、
/// AI は次の呼びかけで戻ってきます。
///
/// **人が「止める」と言ったら、止まること。**それが D29 の約束
/// （人が常に見ていて、**いつでも取り返せる**）の意味です。
///
/// **立て直すには、アプリを立ち上げ直します。**ここで開き直す口は作りません ——
/// 「止めたのに勝手に戻る」を作らないためです。
#[tauri::command]
pub fn mcp_stop(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let Some(endpoint) = app.try_state::<sshboard_mcp::McpEndpoint>() else {
        return Err("MCP は立っていません".to_string());
    };
    endpoint.shutdown();
    Ok(())
}

/// MCP の口がまだ立っているか。**「止める」を出していいかの判断。**
#[tauri::command]
pub fn mcp_running(app: tauri::AppHandle) -> bool {
    use tauri::Manager;
    app.try_state::<sshboard_mcp::McpEndpoint>().is_some()
}

#[tauri::command]
pub fn stop_stream(stream: State<'_, Arc<OutputStream>>) {
    stream.stop();
}

/// この版。**Windows には「〜について」がありません。**
///
/// 自前タイトルバー（D17）にした結果、Windows では OS のメニューごと消えました。
/// **版を確かめる場所がどこにも無い**と実機から言われた分です。
///
/// `CARGO_PKG_VERSION` は 3 か所で揃えており（`src/version.rs` が見張っています）、
/// **配布物の名前・更新の判定と同じ番号**です。
/// **帯を人の手元へ書き出す**（案・4 枚目）。
///
/// ## この口は画面にしかありません
///
/// **MCP には出しません。**落とし先は人の手元で、
/// **囲い（D22）はそこを 1 ミリも守りません** ——
/// D27 で「手元へ落とす口は AI に作らない」と決めたのと同じ性質です。
///
/// AI は `read_stream` で同じ流れを読めます。**ファイルにする必要があるのは人だけ**です。
///
/// ## 中身は画面が組みます
///
/// 実行体は帯を貯めません（控えは画面が持っています）。
/// ここは**人が選んだ場所へ書くだけ**です ——
/// 何を書くかを決めるのは画面で、**人は押す前にその中身を見ています。**
#[tauri::command]
pub fn band_export(destination: String, text: String) -> Result<usize, String> {
    let bytes = text.len();
    std::fs::write(&destination, text).map_err(|error| {
        // **握り潰さない。**書けなかったのに書けたように見せない。
        format!("書き出せません: {error}")
    })?;
    Ok(bytes)
}

/// 報告に貼る素材（案・4 枚目「版情報を写す」）。
///
/// **家の場所を返しません。**`/Users/<名前>` は `~` に置き換えます ——
/// **これは public リポジトリの Issue へ貼られる文**です
/// （product-baseline §25 / `oss-privacy-check` が検出します）。
///
/// 画面側でも落としますが、ここでも落とします ——
/// **受け取らないものは漏らせません。**
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFacts {
    pub version: String,
    /// `macos · aarch64` の形。
    pub platform: String,
    /// 設定の置き場所。**家の部分は `~`。**
    pub config_dir: Option<String>,
    pub mcp_running: bool,
}

#[tauri::command]
pub fn report_facts(app: tauri::AppHandle, path: State<'_, ConnectionsPath>) -> ReportFacts {
    let config_dir = path
        .0
        .parent()
        .map(|dir| without_home(&dir.display().to_string()));
    ReportFacts {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: format!("{} · {}", std::env::consts::OS, std::env::consts::ARCH),
        config_dir,
        mcp_running: mcp_running(app),
    }
}

/// 家の場所を `~` にする。**見つからなければそのまま返します**
/// （壊して短くするより、そのまま出す）。
fn without_home(path: &str) -> String {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| home.to_string_lossy().into_owned());
    match home {
        Some(home) if !home.is_empty() && path.starts_with(&home) => {
            format!("~{}", &path[home.len()..])
        }
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod report_tests {
    use super::without_home;

    #[test]
    fn it_takes_the_home_out() {
        // **public リポへ貼られる文です。**利用者名を入れません。
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }
        let inside = format!("{home}/Library/Application Support/x");
        let out = without_home(&inside);
        assert!(out.starts_with("~/"), "{out}");
        assert!(!out.contains(&home), "{out}");
    }

    #[test]
    fn it_leaves_a_path_outside_the_home_alone() {
        // **壊して短くするより、そのまま出す。**
        assert_eq!(without_home("/opt/sshboard"), "/opt/sshboard");
    }
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
