//! **合言葉を平文で残さないための、薄い中継**（Issue #2 の残り半分）。
//!
//! ## なぜ要るのか
//!
//! MCP は Streamable HTTP で出しています（D15）。繋ぐには
//! `claude mcp add --transport http ... --header "Authorization: Bearer <合言葉>"`
//! と書くことになり、**その合言葉が `~/.claude.json` に平文で残ります。**
//! コマンド履歴にも残ります。
//!
//! これは product-baseline §14「**秘密情報を平文で残すコマンドを人へ案内しない**」に
//! 真っ向からぶつかります。**こちらの見落としでした**（Issue #2 の報告で気づきました）。
//!
//! ## 何をするのか
//!
//! ```text
//! claude mcp add sshboard -- /path/to/sshboard --mcp-stdio-proxy
//! ```
//!
//! この形なら **合言葉はどこにも書きません。**中継が起動時に自分で読みます。
//! ポートが変わっても登録は変わりません。
//!
//! stdin から来た JSON-RPC を、動いている本体の HTTP へ流し、返りを stdout へ返すだけ。
//!
//! ## これは「別プロセスを立てる」ではない（D8 / Issue 001）
//!
//! **SSH を張るのは本体だけです。**この中継は SSH も Engine も持ちません。
//! 帯（Band）にも載りません。**本体が動いていなければ、何もできずに断ります。**
//! 増えるのは「文字を右から左へ渡す」プロセス 1 本で、
//! **見えない SSH セッションは 1 本も増えません**（CLAUDE.md 禁止事項 3）。
//!
//! ## 外へは出られない
//!
//! HTTP の口は TLS 機能を落として持っています（`default-features = false`）。
//! **平文の HTTP しか喋れず、宛先は 127.0.0.1 固定**です。

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::Value;

/// この形で起動されたら、GUI ではなく中継として動く。
pub const FLAG: &str = "--mcp-stdio-proxy";

/// 本体が動いていないときに返す文面。**人が次に何をすればいいかを書く。**
const NOT_RUNNING: &str =
    "sshboard が動いていません。アプリを起動してから、もう一度お試しください。";

/// 合言葉が見つからないときの文面。
const NO_TOKEN: &str = "sshboard の合言葉が見つかりません。アプリを一度起動すると作られます。";

/// 引数に中継の指定があるか。
///
/// **前方一致や部分一致にしません。**`--mcp-stdio-proxy-foo` のような
/// 打ち間違いを黙って受けると、GUI を出すつもりが中継で立ち上がり、
/// **画面が出ないまま固まったように見えます。**
pub fn requested<S: AsRef<str>>(args: &[S]) -> bool {
    args.iter().any(|arg| arg.as_ref() == FLAG)
}

/// JSON-RPC の 1 行から `id` を取り出す。
///
/// **返事には同じ `id` を載せないと、相手は待ち続けます。**
/// 読めない行・`id` の無い行（通知）は `None`。
pub fn request_id(line: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(line).ok()?;
    parsed.get("id").filter(|id| !id.is_null()).cloned()
}

/// エラーの返事を組み立てる。
///
/// **中継が黙って死なないため。**返事が来なければ、相手は理由の分からないまま
/// 待ち続けます。`id` が無い（通知）なら返事は作りません — 仕様上、返してはいけない。
pub fn error_response(id: Option<Value>, message: &str) -> Option<String> {
    let id = id?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        // -32000 は「実装が決めるサーバー側エラー」の範囲。
        "error": { "code": -32000, "message": message },
    });
    Some(body.to_string())
}

/// Server-Sent Events の本文から、JSON の塊だけを取り出す。
///
/// Streamable HTTP は、返事を `application/json` で 1 個返すことも、
/// `text/event-stream` で複数返すこともあります。**両方受けないと繋がりません。**
///
/// 1 つのイベントに `data:` が複数行あるときは、改行で繋ぐのが SSE の決まりです。
pub fn parse_sse(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in body.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);

        if line.is_empty() {
            // 空行 ＝ イベントの終わり。
            push_event(&mut out, std::mem::take(&mut current));
            continue;
        }

        if let Some(rest) = line.strip_prefix("data:") {
            // `data: {...}` の空白 1 つだけを落とす（SSE の決まり）。
            current.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        }
        // `event:` `id:` `retry:` とコメント（`:`）は捨てます。
        // **中身は JSON-RPC で、種類はその中に書いてあります。**
    }

    push_event(&mut out, current);
    out
}

/// 1 つのイベントを積む。**中身が空なら捨てます。**
///
/// サーバーは繋がった直後に、こういう**中身の無いフレーム**を送ってきます（実測）。
///
/// ```text
/// data: \nid: 0\nretry: 3000\n\n
/// ```
///
/// これは SSE の再接続間隔の告知で、**JSON-RPC ではありません。**
/// そのまま流すと stdout に空行が出て、**相手は構文誤りとして読みます。**
/// 部品の試験だけでは出ず、**本体へ通しで繋いで初めて見つかりました。**
fn push_event(out: &mut Vec<String>, data: Vec<String>) {
    let joined = data.join("\n");
    if joined.trim().is_empty() {
        return;
    }
    out.push(joined);
}

/// 合言葉の置き場所。**本体と同じ場所を読むだけで、作りません。**
///
/// 中継がここを作ってしまうと、**本体が使っていない合言葉で待つ**ことになります。
fn token_path() -> Option<PathBuf> {
    let connections = sshboard_connections::default_path().ok()?;
    Some(connections.parent()?.join(crate::mcp_host::TOKEN_FILE))
}

fn read_token() -> Option<String> {
    if let Some(pinned) = std::env::var(crate::mcp_host::TOKEN_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(pinned);
    }
    let held = std::fs::read_to_string(token_path()?).ok()?;
    let held = held.trim().to_string();
    if held.is_empty() {
        None
    } else {
        Some(held)
    }
}

/// 中継として走る。**戻ってきたら、そのまま終わります。**
pub fn run() {
    let Some(token) = read_token() else {
        // stderr は MCP クライアントのログに出ます。**stdout へ書かないこと**
        // — あそこは JSON-RPC の通り道で、混ぜると相手が構文誤りで落ちます。
        eprintln!("[sshboard] {NO_TOKEN}");
        report_startup_failure(NO_TOKEN);
        return;
    };

    let port = crate::mcp_host::port_from_env();
    let url = format!("http://127.0.0.1:{port}{}", sshboard_mcp::MCP_PATH);

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("[sshboard] 中継を動かせません: {error}");
            return;
        }
    };

    runtime.block_on(relay(url, token));
}

/// 起動できなかったことを、**JSON-RPC の作法で 1 回だけ**伝える。
///
/// 何も返さずに終わると、相手には「起動に失敗した」としか出ません。
/// 最初の 1 通に返事を返せば、**理由が人の画面に出ます。**
fn report_startup_failure(message: &str) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let Some(body) = error_response(request_id(&line), message) else {
            continue;
        };
        let _ = writeln!(stdout, "{body}");
        let _ = stdout.flush();
    }
}

async fn relay(url: String, token: String) {
    // **TLS を持たせません**（`default-features = false`）。
    // 宛先は 127.0.0.1 固定で、外へ出る道はありません。
    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("[sshboard] HTTP の口を作れません: {error}");
            return;
        }
    };

    // Streamable HTTP は `initialize` の返事で会期の番号をよこします。
    // **以後それを載せないと、毎回新しい会期になります。**
    let mut session: Option<String> = None;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines().map_while(Result::ok) {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let id = request_id(&line);
        let mut request = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            // **両方を受けると言う。**サーバーはどちらで返すか自分で決めます。
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .body(line);

        if let Some(held) = &session {
            request = request.header("mcp-session-id", held.clone());
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                // **繋がらない理由を、人の言葉で返す。**
                // ここが一番出やすい失敗（本体を起動していない）です。
                eprintln!("[sshboard] {NOT_RUNNING}（{error}）");
                write_line(&mut stdout, error_response(id, NOT_RUNNING));
                continue;
            }
        };

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            // 合言葉が古い＝本体が作り直した、が唯一の筋。
            let message = "sshboard の合言葉が合いません。アプリを再起動してください。";
            eprintln!("[sshboard] {message}");
            write_line(&mut stdout, error_response(id, message));
            continue;
        }

        if let Some(fresh) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
        {
            session = Some(fresh.to_string());
        }

        let is_sse = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));

        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                eprintln!("[sshboard] 返事を読めません: {error}");
                write_line(&mut stdout, error_response(id, "返事を読めませんでした。"));
                continue;
            }
        };

        // 通知（`id` 無し）には、本体は空の 202 を返します。**何も書きません。**
        if body.trim().is_empty() {
            continue;
        }

        if is_sse {
            for message in parse_sse(&body) {
                write_line(&mut stdout, Some(message));
            }
        } else {
            write_line(&mut stdout, Some(body.trim().to_string()));
        }
    }
}

/// stdout へ 1 行書いて、**すぐ流す。**
///
/// 溜めると、相手は返事が来ないまま待ちます。
fn write_line(stdout: &mut std::io::Stdout, body: Option<String>) {
    let Some(body) = body else {
        return;
    };
    if writeln!(stdout, "{body}").is_err() {
        return;
    }
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_is_matched_exactly_and_not_by_prefix() {
        // Arrange & Act & Assert
        assert!(requested(&[FLAG]));
        assert!(requested(&["sshboard", FLAG]));
        assert!(!requested(&["sshboard"]));
        // **打ち間違いを黙って受けない。**受けると画面が出ないまま固まって見えます。
        assert!(!requested(&["--mcp-stdio-proxy-foo"]));
        assert!(!requested(&["--mcp-stdio"]));
    }

    #[test]
    fn the_id_comes_back_out_of_a_request() {
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#),
            Some(serde_json::json!(7))
        );
        // 文字列の id も仕様上ありえます。
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","id":"a","method":"x"}"#),
            Some(serde_json::json!("a"))
        );
    }

    #[test]
    fn a_notification_has_no_id_and_gets_no_reply() {
        // **返事を返してはいけない側。**返すと相手は知らない id を受け取ります。
        assert_eq!(request_id(r#"{"jsonrpc":"2.0","method":"notify"}"#), None);
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","id":null,"method":"x"}"#),
            None
        );
        assert!(error_response(None, "だめ").is_none());
    }

    #[test]
    fn a_line_that_is_not_json_is_refused_rather_than_crashing() {
        // 相手が壊れた行を出しても、中継は落ちない。
        assert_eq!(request_id("これは JSON ではない"), None);
        assert_eq!(request_id(""), None);
    }

    #[test]
    fn an_error_reply_carries_the_same_id_and_a_readable_message() {
        // Arrange & Act
        let body = error_response(Some(serde_json::json!(3)), NOT_RUNNING).expect("返事が無い");
        let parsed: Value = serde_json::from_str(&body).expect("読めない");

        // Assert
        assert_eq!(parsed["id"], serde_json::json!(3));
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["error"]["message"], NOT_RUNNING);
        // **合言葉が返事に混ざらないこと。**
        assert!(!body.contains("Bearer"));
    }

    #[test]
    fn one_sse_event_yields_one_message() {
        // Arrange
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1}\n\n";

        // Act & Assert
        assert_eq!(parse_sse(body), vec![r#"{"jsonrpc":"2.0","id":1}"#]);
    }

    #[test]
    fn several_events_come_back_in_order() {
        // Arrange
        let body = "data: {\"id\":1}\n\ndata: {\"id\":2}\n\n";

        // Act & Assert
        assert_eq!(parse_sse(body), vec![r#"{"id":1}"#, r#"{"id":2}"#]);
    }

    #[test]
    fn a_data_field_split_over_lines_is_joined_with_newlines() {
        // SSE の決まり。**繋がないと JSON として壊れます。**
        let body = "data: {\ndata: \"id\": 1\ndata: }\n\n";
        assert_eq!(parse_sse(body), vec!["{\n\"id\": 1\n}"]);
    }

    #[test]
    fn carriage_returns_and_other_fields_do_not_reach_the_output() {
        // Arrange — Windows の改行と、捨てるべき欄。
        let body = ": ping\r\nevent: message\r\nid: 9\r\ndata: {\"id\":1}\r\n\r\n";

        // Act & Assert
        assert_eq!(parse_sse(body), vec![r#"{"id":1}"#]);
    }

    #[test]
    fn a_last_event_without_a_trailing_blank_line_is_still_returned() {
        // 切れた本文を黙って捨てない。
        assert_eq!(parse_sse("data: {\"id\":1}"), vec![r#"{"id":1}"#]);
    }

    #[test]
    fn the_servers_opening_retry_frame_is_not_passed_through() {
        // Arrange — **本体が実際に最初に送ってくるフレーム**（実測・0.1.6）。
        // これを流すと stdout に空行が出て、相手は構文誤りとして読みます。
        let body = "data: \nid: 0\nretry: 3000\n\ndata: {\"id\":1}\n\n";

        // Act & Assert — **JSON の塊だけが出ること。**
        assert_eq!(parse_sse(body), vec![r#"{"id":1}"#]);
    }

    #[test]
    fn an_empty_body_yields_nothing() {
        assert!(parse_sse("").is_empty());
        assert!(parse_sse("\n\n").is_empty());
    }
}
