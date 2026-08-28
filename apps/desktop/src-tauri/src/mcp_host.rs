//! MCP をこのプロセスの中で立てる（decisions D8 / D15）。
//!
//! **別バイナリにしない。別プロセスにしない。**
//! GUI と MCP が同じ帯・同じ Operation Engine を共有することが製品の前提（PRD §4-1）。

use std::sync::Arc;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_credentials::SecretStore;
use sshboard_engine::Engine;
use sshboard_mcp::DEFAULT_ACK_TIMEOUT;
use sshboard_stream::OutputStream;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{McpAccess, McpUrl};

/// 画面が待ち受けるイベント名。
pub const MCP_READY_EVENT: &str = "mcp://ready";

/// 合言葉を置く OS ストアの区分と名前（D11 / D23）。**ファイルには書かない。**
const KEYRING_SERVICE: &str = "sshboard";
const TOKEN_REF: &str = "mcp-token";

/// MCP の合言葉を決める。
///
/// **起動ごとに変わると、人が毎回 `claude mcp add` をやり直すことになる。**
/// 面倒を減らすために作っている道具が、新しい面倒を足しては本末転倒なので、
/// OS の資格情報ストアへ置いて使い回す。
///
/// **読めない・書けない環境では、起動ごとの合言葉へ落とす。**
/// 弱い合言葉で開けたことにはしない。落ちたことは端末へ出す。
fn resolve_token() -> Option<String> {
    let store = SecretStore::new(KEYRING_SERVICE);

    if let Ok(held) = store.get(TOKEN_REF) {
        if !held.trim().is_empty() {
            return Some(held);
        }
    }

    let fresh = sshboard_mcp::new_token();
    match store.put(TOKEN_REF, &fresh) {
        Ok(()) => Some(fresh),
        Err(error) => {
            eprintln!(
                "[sshboard] 合言葉を OS のストアへ置けません（{error}）。\
                 この起動限りの合言葉を使います。**起動するたびに貼り直しが要ります。**"
            );
            None
        }
    }
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
        eprintln!("[sshboard] トークンは画面の「MCP」から取ってください（起動ごとに変わります）");

        app.state::<McpUrl>().set(url.clone(), token.clone());

        if let Err(error) = app.emit(MCP_READY_EVENT, McpAccess { url, token }) {
            eprintln!("[sshboard] MCP の口を画面へ渡せません: {error}");
        }

        // 止められるように持っておく。
        app.manage(endpoint);
    });
}
