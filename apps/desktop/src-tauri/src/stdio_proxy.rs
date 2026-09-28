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

/// 1 本ぶんの往復。**返事が来たら、その場で書き出します。**
///
/// **呼びごとに分けてあります**（実運用の指摘・2026-09-28）——
/// 前は `for` の中で `await` していたので、**遅い呼び 1 本で後ろが全部止まりました。**
async fn forward(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &std::sync::Mutex<Option<String>>,
    out: &std::sync::Mutex<std::io::Stdout>,
    line: String,
) {
    let id = request_id(&line);
    let mut request = client
        .post(url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        // **両方を受けると言う。**サーバーはどちらで返すか自分で決めます。
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .body(line);

    if let Some(held) = session.lock().ok().and_then(|held| held.clone()) {
        request = request.header("mcp-session-id", held);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            // **繋がらない理由を、人の言葉で返す。**
            // ここが一番出やすい失敗（本体を起動していない）です。
            eprintln!("[sshboard] {NOT_RUNNING}（{error}）");
            say(out, error_response(id, NOT_RUNNING));
            return;
        }
    };

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        // 合言葉が古い＝本体が作り直した、が唯一の筋。
        let message = "sshboard の合言葉が合いません。アプリを再起動してください。";
        eprintln!("[sshboard] {message}");
        say(out, error_response(id, message));
        return;
    }

    if let Some(fresh) = response
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
    {
        if let Ok(mut held) = session.lock() {
            *held = Some(fresh.to_string());
        }
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
            say(out, error_response(id, "返事を読めませんでした。"));
            return;
        }
    };

    // 通知（`id` 無し）には、本体は空の 202 を返します。**何も書きません。**
    if body.trim().is_empty() {
        return;
    }

    if is_sse {
        for message in parse_sse(&body) {
            say(out, Some(message));
        }
    } else {
        say(out, Some(body.trim().to_string()));
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
    let session = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let out = std::sync::Arc::new(std::sync::Mutex::new(std::io::stdout()));

    // **stdin を読むのは、別の糸。**
    //
    // ここで直接読むと、**読んでいる間、裏の呼びが 1 つも進みません**
    // （走らせ方が `new_current_thread` なので）。
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if tx.send(line).is_err() {
                return;
            }
        }
    });

    pump(
        rx,
        std::sync::Arc::new(client),
        std::sync::Arc::new(url),
        std::sync::Arc::new(token),
        session,
        out,
    )
    .await;
}

/// 来た行を流す。**会期の番号が決まったあとは、並べて流します。**
///
/// **`relay` から切り出してあります**（2026-09-28）—— ここが
/// 「1 本ずつしか流れない」所だったので、**試験から直に回せる形**にしました。
/// 呼ぶ側が並行にしてしまう試験では、**直した所を見たことになりません。**
async fn pump(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<String>,
    client: std::sync::Arc<reqwest::Client>,
    url: std::sync::Arc<String>,
    token: std::sync::Arc<String>,
    session: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    out: std::sync::Arc<std::sync::Mutex<std::io::Stdout>>,
) {
    let mut running = Vec::new();
    while let Some(line) = rx.recv().await {
        // **会期の番号が決まるまでは、1 本ずつ。**
        //
        // `initialize` の返事で番号が来ます。それより先に別の呼びを投げると、
        // **番号を載せられず、毎回新しい会期**になります。
        // 決まったあとは**並べて流します** —— そこが詰まっていた所です。
        let known = session.lock().ok().and_then(|held| held.clone()).is_some();
        if !known {
            forward(&client, &url, &token, &session, &out, line).await;
            continue;
        }

        let client = std::sync::Arc::clone(&client);
        let token = std::sync::Arc::clone(&token);
        let url = std::sync::Arc::clone(&url);
        let session = std::sync::Arc::clone(&session);
        let out = std::sync::Arc::clone(&out);
        running.push(tokio::spawn(async move {
            forward(&client, &url, &token, &session, &out, line).await;
        }));
    }

    // **口が閉じたら、走っている分を待ち切る。**
    // 途中で落とすと、**返事を出さないまま終わった呼び**が残ります。
    for task in running {
        let _ = task.await;
    }
}

/// stdout へ 1 行書いて、**すぐ流す。**
///
/// 溜めると、相手は返事が来ないまま待ちます。
///
/// **錠を掛けます**（2026-09-28）—— 並べて流すようにしたので、
/// **2 本の返事が混ざって 1 行になる**と、相手は構文誤りで落ちます。
fn say(out: &std::sync::Mutex<std::io::Stdout>, body: Option<String>) {
    let Some(body) = body else {
        return;
    };
    let Ok(mut stdout) = out.lock() else {
        return;
    };
    if writeln!(stdout, "{body}").is_err() {
        return;
    }
    let _ = stdout.flush();
}

#[cfg(test)]
mod concurrency_tests {
    //! **遅い呼び 1 本で、後ろが全部止まらないこと**（実運用の指摘・2026-09-28）。
    //!
    //! 実機の報告 ——
    //!
    //! > 利用者が画面から同じ接続に繋ぎ、端末とファイルの両方が使える状態になりました。
    //! > そのあとで MCP の `session_status` を呼びましたが、**120 秒以上返りません**。
    //! > …**proxy とアプリ本体の間で呼び出しが 1 本ずつしか流れず、
    //! > 詰まっている可能性はありませんか。**
    //!
    //! **在りました。**中継は
    //!
    //! ```text
    //! for line in stdin.lines() { … request.send().await; write_line(…); }
    //! ```
    //!
    //! という形で、**次の行は、前の返事が来るまで読まれません。**
    //!
    //! **20 秒の打ち切り（b4cf6d3）では解けません** —— `await_answer` は
    //! **最大 300 秒**待ちます。**「待てる口」を入れた結果、待っている間
    //! 「気づく口」が使えなくなる**、という裏返しになっていました。
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    /// **呼びが届いた時刻を控える、使い捨ての相手。**
    ///
    /// 本文に `slow` が入っていたら 2 秒待ってから返します。
    ///
    /// **測るのは「全体で何秒か」ではありません。**それだと**どちらでも約 2 秒**で、
    /// **1 本ずつでも通ってしまいます**（実際に 2 回、通る試験を書きました）。
    /// **2 本目の呼びが、いつ届いたか**を見ます ——
    /// 1 本ずつなら**遅いほうが終わってから**、並べて流していれば**すぐ**です。
    type Arrivals = std::sync::Arc<std::sync::Mutex<Vec<(String, Duration)>>>;

    fn recording_server() -> (String, Arrivals) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("口を開けない");
        let port = listener.local_addr().unwrap().port();
        let arrivals: Arrivals = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = std::sync::Arc::clone(&arrivals);
        let born = Instant::now();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let Ok(mut stream) = stream else { continue };
                let seen = std::sync::Arc::clone(&seen);
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut length = 0usize;
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        if let Some(rest) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            length = rest.trim().parse().unwrap_or(0);
                        }
                        if line == "\r\n" || line == "\n" {
                            break;
                        }
                    }
                    let mut body = vec![0u8; length];
                    use std::io::Read;
                    let _ = reader.read_exact(&mut body);
                    let body = String::from_utf8_lossy(&body).to_string();
                    let which = if body.contains("slow") {
                        "slow"
                    } else {
                        "fast"
                    };
                    // **届いた時刻を控える。**ここが見たいものです。
                    if let Ok(mut held) = seen.lock() {
                        held.push((which.to_string(), born.elapsed()));
                    }
                    if which == "slow" {
                        std::thread::sleep(Duration::from_secs(2));
                    }
                    let payload = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                        payload.len()
                    );
                    let _ = stream.flush();
                });
            }
        });
        (format!("http://127.0.0.1:{port}/mcp"), arrivals)
    }

    #[test]
    fn a_slow_call_does_not_hold_up_the_ones_behind_it() {
        // **`pump` を直に回します。**`tokio::join!` で包むと、
        // **呼ぶ側が並行にしてしまい、直した所を見たことになりません。**
        let (url, arrivals) = recording_server();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        runtime.block_on(async {
            let client = std::sync::Arc::new(reqwest::Client::builder().build().expect("client"));
            // **会期の番号は決まっている**ことにします（`initialize` の後の状態）。
            let session = std::sync::Arc::new(std::sync::Mutex::new(Some("s1".to_string())));
            let out = std::sync::Arc::new(std::sync::Mutex::new(std::io::stdout()));
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // **遅いほうを先に入れます。**後ろが止まるなら、ここで詰まります。
            tx.send(r#"{"jsonrpc":"2.0","id":1,"method":"slow"}"#.to_string())
                .unwrap();
            tx.send(r#"{"jsonrpc":"2.0","id":2,"method":"fast"}"#.to_string())
                .unwrap();
            drop(tx);

            super::pump(
                rx,
                client,
                std::sync::Arc::new(url),
                std::sync::Arc::new("test".to_string()),
                session,
                out,
            )
            .await;
        });

        let held = arrivals.lock().unwrap().clone();
        let at = |name: &str| {
            held.iter()
                .find(|(which, _)| which == name)
                .map(|(_, at)| *at)
                .unwrap_or_else(|| panic!("{name} が届いていません: {held:?}"))
        };
        // **絶対の時刻ではなく、2 本の差を見ます。**
        // 始まりが何秒ずれても、**1 本ずつなら差が 2 秒開きます。**
        let gap = at("fast").saturating_sub(at("slow"));

        assert!(
            gap < Duration::from_millis(500),
            "**2 本目が、遅い呼びの後ろで待たされています**: 差が {gap:?}。\
             届いた順と時刻: {held:?}。\
             実機では、これが `session_status` が 120 秒返らない形になりました"
        );
    }
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
