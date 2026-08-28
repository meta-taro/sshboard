//! MCP をこのプロセスの中で立てる（decisions D8 / D15）。
//!
//! **別バイナリにしない。別プロセスにしない。**
//! GUI と MCP が同じ帯・同じ Operation Engine を共有することが製品の前提（PRD §4-1）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_engine::Engine;
use sshboard_mcp::DEFAULT_ACK_TIMEOUT;
use sshboard_stream::OutputStream;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{McpAccess, McpUrl};

/// 画面が待ち受けるイベント名。
pub const MCP_READY_EVENT: &str = "mcp://ready";

/// 合言葉を人が固定したいときの環境変数。**設定ファイルより優先する。**
const TOKEN_ENV: &str = "SSHBOARD_MCP_TOKEN";

/// 合言葉を置くファイル名。接続一覧の隣。
const TOKEN_FILE: &str = "mcp-token";

/// MCP の合言葉を決める（D23）。
///
/// **起動ごとに変わると、人が毎回 `claude mcp add` をやり直すことになる。**
/// 面倒を減らすための道具が新しい面倒を足しては本末転倒なので、使い回す。
///
/// **OS の資格情報ストアには置かない。**あそこは「リモートを開ける秘密」の場所
/// （鍵のパスフレーズ・D11）で、macOS ではバイナリごとに承認を求める。
/// 開発中はビルドのたびに別のバイナリになるため、**承認が毎回出る**（実測）。
/// MCP の合言葉は loopback の取っ手で、露出度は `connections.toml` と同じなので、
/// **同じ置き場所・同じ権限（0600）**に置く。
///
/// 置けない環境では、起動ごとの合言葉へ落とす。**弱い合言葉で開けたことにはしない。**
fn resolve_token() -> Option<String> {
    // 人が固定したいならそれに従う。**製品が上書きしない。**
    if let Some(pinned) = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return Some(pinned);
    }

    let Some(path) = token_path() else {
        eprintln!(
            "[sshboard] 合言葉の置き場所が分かりません。この起動限りの合言葉を使います。\
             **起動するたびに貼り直しが要ります。**"
        );
        return None;
    };

    if let Ok(held) = std::fs::read_to_string(&path) {
        let held = held.trim().to_string();
        if !held.is_empty() {
            return Some(held);
        }
    }

    let fresh = sshboard_mcp::new_token();
    match write_private(&path, &fresh) {
        Ok(()) => Some(fresh),
        Err(error) => {
            eprintln!(
                "[sshboard] 合言葉を保存できません（{error}）。この起動限りの合言葉を使います。\
                 **起動するたびに貼り直しが要ります。**"
            );
            None
        }
    }
}

fn token_path() -> Option<PathBuf> {
    let connections = sshboard_connections::default_path().ok()?;
    Some(connections.parent()?.join(TOKEN_FILE))
}

/// 本人だけが読める形で書く。**接続一覧と同じ扱い。**
fn write_private(path: &Path, value: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, value)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// 0 = OS に空きポートを選ばせる。
/// **番号の扱いはまだ決まっていない**（D15）。MCP クライアントへ登録する形を
/// 実際に試してから決める。
const MCP_PORT: u16 = 0;

pub fn spawn(
    app: AppHandle,
    band: Band,
    stream: Arc<OutputStream>,
    connections_watch: Arc<ConnectionsWatch>,
    engine: Arc<Engine>,
) {
    tauri::async_runtime::spawn(async move {
        let endpoint = match sshboard_mcp::serve(
            band,
            stream,
            connections_watch,
            Some(engine),
            resolve_token(),
            MCP_PORT,
            DEFAULT_ACK_TIMEOUT,
        )
        .await
        {
            Ok(endpoint) => endpoint,
            Err(error) => {
                // 立たなかったことを黙らない。GUI だけ動いて MCP が死んでいる状態が
                // 一番分かりにくい。
                eprintln!("[sshboard] MCP を立ち上げられません: {error}");
                return;
            }
        };

        let url = endpoint.url();
        let token = endpoint.token().to_string();

        // 端末にも出す。MCP クライアントへ登録するときに、画面を見ずに拾えるようにする。
        // **接続先ではなく loopback のポートなので、伏せる対象ではない**（PRD §8）。
        // **合言葉はここに出さない。**端末の記録に残る（product-baseline §14）。
        // 画面で見せて、人が手元で貼る。
        eprintln!("[sshboard] MCP listening on {url}");
        eprintln!("[sshboard] トークンは画面の「MCP」ボタンから写せます");

        app.state::<McpUrl>().set(url.clone(), token.clone());

        if let Err(error) = app.emit(MCP_READY_EVENT, McpAccess { url, token }) {
            eprintln!("[sshboard] MCP の口を画面へ渡せません: {error}");
        }

        // 止められるように持っておく。
        app.manage(endpoint);
    });
}
