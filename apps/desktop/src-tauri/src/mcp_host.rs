//! MCP をこのプロセスの中で立てる（decisions D8 / D15）。
//!
//! **別バイナリにしない。別プロセスにしない。**
//! GUI と MCP が同じ帯・同じ Operation Engine を共有することが製品の前提（PRD §4-1）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::Band;
use sshboard_connections::ConnectionsWatch;
use sshboard_engine::Engine;
use sshboard_mcp::{ServeParts, DEFAULT_ACK_TIMEOUT};
use sshboard_stream::OutputStream;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{McpAccess, McpUrl};

/// 画面が待ち受けるイベント名。
pub const MCP_READY_EVENT: &str = "mcp://ready";

/// **立ち上がらなかったことを画面へ伝える。**
///
/// 以前はここで `eprintln!` して黙って戻っていたため、
/// **画面は「起動中」のまま止まり、人には何も分かりませんでした。**
/// 番号を固定した分、ぶつかる場面が現実に増えるので、必ず出します。
pub const MCP_FAILED_EVENT: &str = "mcp://failed";

/// 合言葉を人が固定したいときの環境変数。**設定ファイルより優先する。**
pub const TOKEN_ENV: &str = "SSHBOARD_MCP_TOKEN";

/// 合言葉を置くファイル名。接続一覧の隣。
pub const TOKEN_FILE: &str = "mcp-token";

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

/// MCP が待つポート。**固定**（D33）。
///
/// 以前は 0（OS に空きを選ばせる）でした。**起動のたびに番号が変わるため、
/// `claude mcp add` をやり直すことになり、`.mcp.json` にも書けませんでした。**
/// 合言葉を使い回す理由（上の `resolve_token`）と同じ理屈が、番号にも当てはまります。
///
/// **22022 を選んだ理由:** SSH の 22 から辿れて覚えやすく、
/// Linux（32768〜）と Windows（49152〜）の一時ポート範囲より下なので、
/// **OS が外向き接続に割り当てて偶発的にぶつかることがありません。**
pub const DEFAULT_MCP_PORT: u16 = 22022;

/// **番号の条件はビルド時に見張る。**実行時の assert では、
/// 走らせるまで気づけない。ここが崩れたら**コンパイルが通りません。**
///
/// - `0` に戻すと、起動ごとに番号が変わる（D33 が消える）
/// - `32768` 以上は Linux の、`49152` 以上は Windows の一時ポート範囲。
///   **OS が外向き接続へ割り当てて、ぶつかる日が出る**
/// - `1024` 未満は特権ポート
const _: () = {
    assert!(DEFAULT_MCP_PORT != 0, "OS 任せに戻っている（D33）");
    assert!(DEFAULT_MCP_PORT >= 1024, "特権ポートに入っている");
    assert!(DEFAULT_MCP_PORT < 32768, "一時ポート範囲に入っている");
};

/// ぶつかった人が動かすための逃げ道。**黙って別の番号に落ちない**ため、
/// 移すのは人の明示的な操作にします。
const PORT_ENV: &str = "SSHBOARD_MCP_PORT";

/// 環境変数からポートを決める。
///
/// **読めない値を黙って既定へ倒しません。**倒すと、人は自分が指定したつもりの
/// 番号で待っていて、実際は別の番号で立っている状態になります。
/// 呼び出し側が理由を出したうえで既定を使います。
///
/// `0` は明示的に許します（OS 任せ）。**試験や、どうしても固定できない環境のため。**
fn resolve_port(raw: Option<&str>) -> Result<u16, String> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_MCP_PORT);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_MCP_PORT);
    }
    trimmed
        .parse::<u16>()
        .map_err(|_| format!("{PORT_ENV} の値を番号として読めません: {trimmed}"))
}

/// 環境変数を見てポートを決める。**本体と中継（`stdio_proxy`）が同じ答えを出すため、
/// ここ 1 か所に置きます。**片方だけ環境変数を見ると、繋がらない組み合わせが生まれます。
///
/// **読めない指定は黙って倒さず、理由を出してから既定へ戻します。**
pub fn port_from_env() -> u16 {
    match resolve_port(std::env::var(PORT_ENV).ok().as_deref()) {
        Ok(port) => port,
        Err(reason) => {
            eprintln!("[sshboard] {reason}（既定の {DEFAULT_MCP_PORT} を使います）");
            DEFAULT_MCP_PORT
        }
    }
}

pub fn spawn(
    app: AppHandle,
    band: Band,
    stream: Arc<OutputStream>,
    connections_watch: Arc<ConnectionsWatch>,
    engine: Arc<Engine>,
) {
    tauri::async_runtime::spawn(async move {
        // **画面を撮る口を差す**（D26）。AI が自分で崩れを見つけられるように。
        // 伏せるのは画面側で、**伏せ終わってから撮る**（capture.rs）。
        let capture = crate::capture::TauriCapture::new(app.clone());

        let port = port_from_env();

        let endpoint = match sshboard_mcp::serve(ServeParts {
            band,
            stream,
            connections_watch,
            engine: Some(engine),
            capture: Some(capture),
            token: resolve_token(),
            port,
            ack_timeout: DEFAULT_ACK_TIMEOUT,
        })
        .await
        {
            Ok(endpoint) => endpoint,
            Err(error) => {
                // 立たなかったことを黙らない。GUI だけ動いて MCP が死んでいる状態が
                // 一番分かりにくい。**端末だけでなく画面にも出す。**
                // 画面へ出さなかったので、「起動中」のまま止まって見えていた。
                eprintln!("[sshboard] MCP を立ち上げられません（ポート {port}）: {error}");
                if let Err(emit_error) = app.emit(
                    MCP_FAILED_EVENT,
                    McpFailure {
                        port,
                        detail: error.to_string(),
                    },
                ) {
                    eprintln!("[sshboard] MCP の失敗を画面へ渡せません: {emit_error}");
                }
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

/// 立ち上がらなかったことを画面へ渡す形。
///
/// **番号を出します。**「ぶつかっている」と言われても、どの番号かが分からないと
/// 人は調べようがありません。**接続先ではなく loopback の番号なので、
/// 伏せる対象ではありません**（PRD §8）。
#[derive(Clone, serde::Serialize)]
pub struct McpFailure {
    pub port: u16,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_nothing_set_the_port_is_the_fixed_one() {
        // **これが D33 の芯。**0（OS 任せ）に戻ったら、
        // `claude mcp add` のやり直しが復活する。
        // 番号そのものの条件は下の `const` で見張っている（**ビルドが止まる**）。
        assert_eq!(resolve_port(None), Ok(DEFAULT_MCP_PORT));
    }

    #[test]
    fn an_empty_value_is_treated_as_unset() {
        assert_eq!(resolve_port(Some("")), Ok(DEFAULT_MCP_PORT));
        assert_eq!(resolve_port(Some("   ")), Ok(DEFAULT_MCP_PORT));
    }

    #[test]
    fn a_number_the_person_set_is_used_as_given() {
        assert_eq!(resolve_port(Some("31000")), Ok(31000));
        assert_eq!(resolve_port(Some(" 31000 ")), Ok(31000));
    }

    #[test]
    fn zero_stays_allowed_because_someone_may_have_to_let_the_os_pick() {
        assert_eq!(resolve_port(Some("0")), Ok(0));
    }

    #[test]
    fn a_value_that_is_not_a_port_is_refused_rather_than_silently_ignored() {
        // **黙って既定へ倒さない。**倒すと、人は指定したつもりの番号で待ち、
        // 実際は別の番号で立っている状態になる。
        assert!(resolve_port(Some("あいうえお")).is_err());
        assert!(resolve_port(Some("70000")).is_err(), "u16 を超えている");
        assert!(resolve_port(Some("-1")).is_err());
    }
}
