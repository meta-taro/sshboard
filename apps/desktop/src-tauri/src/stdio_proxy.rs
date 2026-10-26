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

/// 1 行を本体へ送り、**相手（stdin の向こう）へ書く行**を返す。
///
/// 前は `for` の中で `await` していたので、**遅い呼び 1 本で後ろが全部止まりました。**
/// （並べるのは `pump` の役目。ここは 1 本ぶんだけを扱います。）
async fn forward(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &std::sync::Mutex<Option<String>>,
    hello: &std::sync::Mutex<Option<String>>,
    out: &std::sync::Mutex<std::io::Stdout>,
    line: String,
) {
    if let Some(lines) = exchange(client, url, token, session, hello, line).await {
        for line in lines {
            say(out, Some(line));
        }
    }
}

/// 送る。**会期の番号があれば載せる。**返事の番号が来たら控える。
async fn send(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: Option<String>,
    body: String,
) -> reqwest::Result<reqwest::Response> {
    let mut request = client
        .post(url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        // **両方を受けると言う。**サーバーはどちらで返すか自分で決めます。
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .body(body);
    if let Some(held) = session {
        request = request.header("mcp-session-id", held);
    }
    request.send().await
}

fn session_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// **会期を張り直す**（本体が立ち上がり直した・2026-10-26）。
///
/// 相手が最初に送った `initialize` をそのまま送り直し、新しい番号を受け取って、
/// `notifications/initialized` を送ります（MCP の始まりの手順）。張れたら `true`。
async fn reopen(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &std::sync::Mutex<Option<String>>,
    hello: &std::sync::Mutex<Option<String>>,
) -> bool {
    let Some(opening) = hello.lock().ok().and_then(|held| held.clone()) else {
        return false;
    };
    let Ok(response) = send(client, url, token, None, opening).await else {
        return false;
    };
    let Some(fresh) = response
        .status()
        .is_success()
        .then(|| session_of(&response))
        .flatten()
    else {
        return false;
    };
    // 返事の中身（相手は既に受け取っている）は読み捨てる。
    let _ = response.text().await;
    if let Ok(mut held) = session.lock() {
        *held = Some(fresh.clone());
    }
    let initialized = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string();
    send(client, url, token, Some(fresh), initialized)
        .await
        .is_ok_and(|done| done.status().is_success())
}

/// 1 本ぶんのやりとり。**相手へ返す行**を返します（通知なら `None`）。
///
/// **呼びには必ず答えます**（`id` があるもの）。素の文字や空を流すと、相手はその呼びの
/// 返事を受け取れず、**時間切れまで待ち続けます**（実機・2026-10-26 の 120 秒）。
async fn exchange(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &std::sync::Mutex<Option<String>>,
    hello: &std::sync::Mutex<Option<String>>,
    line: String,
) -> Option<Vec<String>> {
    let id = request_id(&line);
    let mut retried = false;
    loop {
        let held = session.lock().ok().and_then(|held| held.clone());
        let response = match send(client, url, token, held.clone(), line.clone()).await {
            Ok(response) => response,
            Err(error) => {
                // **繋がらない理由を、人の言葉で返す。**
                // ここが一番出やすい失敗（本体を起動していない）です。
                eprintln!("[sshboard] {NOT_RUNNING}（{error}）");
                return error_response(id, NOT_RUNNING).map(|line| vec![line]);
            }
        };

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            // 合言葉が古い＝本体が作り直した、が唯一の筋。
            let message = "sshboard の合言葉が合いません。アプリを再起動してください。";
            eprintln!("[sshboard] {message}");
            return error_response(id, message).map(|line| vec![line]);
        }

        // **知らない会期**＝本体が立ち上がり直した。1 回だけ張り直して、送り直す。
        if response.status() == reqwest::StatusCode::NOT_FOUND && held.is_some() && !retried {
            retried = true;
            if let Ok(mut lost) = session.lock() {
                *lost = None;
            }
            if reopen(client, url, token, session, hello).await {
                eprintln!("[sshboard] 本体が立ち上がり直したので、会期を張り直しました");
                continue;
            }
            let message = "sshboard が立ち上がり直しました。会期を張り直せなかったので、\
                           /mcp で繋ぎ直してください。";
            eprintln!("[sshboard] {message}");
            return error_response(id, message).map(|line| vec![line]);
        }

        if let Some(fresh) = session_of(&response) {
            if let Ok(mut held) = session.lock() {
                *held = Some(fresh);
            }
        }

        let status = response.status();
        let is_sse = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));

        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                eprintln!("[sshboard] 返事を読めません: {error}");
                return error_response(id, "返事を読めませんでした。").map(|line| vec![line]);
            }
        };

        // 通知（`id` 無し）には、本体は空の 202 を返します。**何も書きません。**
        if body.trim().is_empty() && id.is_none() {
            return None;
        }

        // **失敗の素の文字を、そのまま流さない。**JSON-RPC の失敗として返す。
        if !status.is_success() && !is_sse && serde_json::from_str::<Value>(body.trim()).is_err() {
            let message = format!("sshboard が {status} を返しました: {}", body.trim());
            eprintln!("[sshboard] {message}");
            return error_response(id, &message).map(|line| vec![line]);
        }

        let lines = if is_sse {
            parse_sse(&body)
        } else {
            vec![body.trim().to_string()]
        };
        return Some(lines.into_iter().filter(|line| !line.is_empty()).collect());
    }
}

/// 繋ぎに行って返事が無い相手を、これだけ待って諦める。
///
/// **呼び全体の締め切りではありません。**`read_file` や `run_operation` は
/// 正しく何十秒もかかります。**切るのは「相手がそこに居るか」だけ**です。
const REACH_GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(3);

/// 送る口を作る。
///
/// **TLS を持たせません**（`default-features = false`）。
/// 宛先は 127.0.0.1 固定で、外へ出る道はありません。
///
/// **時間切れが 1 つも無い口は、黙って待ち続けます。**
/// 実運用（2026-09-28）で「ローカルだけで答えられる呼びが 120 秒返らない」と
/// 報告されました。**誰も聞いていない口なら拒否がその場で返ります**が、
/// 繋ぎに行って返事が無いときは、待つ以外に何もしていませんでした。
fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(REACH_GIVE_UP_AFTER)
        .build()
}

async fn relay(url: String, token: String) {
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("[sshboard] HTTP の口を作れません: {error}");
            return;
        }
    };

    // Streamable HTTP は `initialize` の返事で会期の番号をよこします。
    // **以後それを載せないと、毎回新しい会期になります。**
    let session = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    // **相手が最初に送った `initialize`。**本体が立ち上がり直したとき、これで張り直します。
    let hello = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let out = std::sync::Arc::new(std::sync::Mutex::new(std::io::stdout()));

    // **stdin を読むのは、別の糸。**
    //
    // ここで直接読むと、**読んでいる間、裏の呼びが 1 つも進みません**
    // （走らせ方が `new_current_thread` なので）。
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
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
        hello,
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
    hello: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    out: std::sync::Arc<std::sync::Mutex<std::io::Stdout>>,
) {
    let mut running = Vec::new();
    while let Some(line) = rx.recv().await {
        // **始まりの挨拶を控える**（張り直すときに送り直すため）。
        if serde_json::from_str::<Value>(&line)
            .ok()
            .is_some_and(|message| message["method"] == "initialize")
        {
            if let Ok(mut held) = hello.lock() {
                *held = Some(line.clone());
            }
        }
        // **会期の番号が決まるまでは、1 本ずつ。**
        //
        // `initialize` の返事で番号が来ます。それより先に別の呼びを投げると、
        // **番号を載せられず、毎回新しい会期**になります。
        // 決まったあとは**並べて流します** —— そこが詰まっていた所です。
        let known = session.lock().ok().and_then(|held| held.clone()).is_some();
        if !known {
            forward(&client, &url, &token, &session, &hello, &out, line).await;
            continue;
        }

        let client = std::sync::Arc::clone(&client);
        let token = std::sync::Arc::clone(&token);
        let url = std::sync::Arc::clone(&url);
        let session = std::sync::Arc::clone(&session);
        let hello = std::sync::Arc::clone(&hello);
        let out = std::sync::Arc::clone(&out);
        running.push(tokio::spawn(async move {
            forward(&client, &url, &token, &session, &hello, &out, line).await;
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
                std::sync::Arc::new(std::sync::Mutex::new(None)),
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

    /// **返事が来ない相手を、いつまでも待たない。**
    ///
    /// 実運用の報告（2026-09-28）——
    ///
    /// > `session_status` や `list_readonly_commands` のように、ローカルだけで
    /// > 答えられる呼び出しでも 120 秒を超えて返りませんでした
    ///
    /// **断られる筋は、既にその場で返っていました**（誰も聞いていない口へ繋ぐと
    /// 拒否が返り、`NOT_RUNNING` を出します）。**無限に待てたのは、繋ぎに行って
    /// 返事が無い筋だけ**です —— 時間切れが 1 つも付いていませんでした。
    #[test]
    fn a_silent_address_is_given_up_on_instead_of_waited_out() {
        // 192.0.2.1 は TEST-NET（RFC 5737）。**外へ出ず、返事も来ません。**
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        let started = Instant::now();
        let answer = runtime.block_on(async {
            let client = super::http_client().expect("client");
            client
                .post("http://192.0.2.1:9/mcp")
                .body("{}")
                .send()
                .await
        });
        let waited = started.elapsed();

        assert!(answer.is_err(), "黒い穴から返事が来ました");
        // **見ているのは「いつまでも待たない」こと**（実機では 120 秒だった）。
        // 8 秒にしていた頃は、他の試験と並べて走らせると繋ぎの準備だけで 8 秒を超え、
        // 何も壊れていないのに落ちていました（2026-10-26・3 回）。120 秒の待ちは 30 秒でも捕まります。
        assert!(
            waited < Duration::from_secs(30),
            "**返事の無い相手を {waited:?} 待ちました。**\
             実機では、これが 120 秒返らない形になります"
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

/// **本体が立ち上がり直したら、会期を張り直す**（実機・2026-10-26）。
///
/// 本体が立ち上がり直す（自動更新の再起動を含む）と、中継が持っている会期の番号は
/// 本体にとって知らないものになり、`404 Session not found`（素の文字）が返ります。
/// 中継はそれを**そのまま返事として流していました** —— JSON-RPC ではないので、
/// 相手（Claude Code）はその呼びの返事を受け取れず、**120 秒待って時間切れ**でした。
/// `/mcp` で繋ぎ直すと中継ごと作り直されるので、毎回それで戻っていました。
#[cfg(test)]
mod session_tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    type Seen = std::sync::Arc<std::sync::Mutex<Vec<(String, Option<String>)>>>;

    /// 会期を持つ本体のまね。**`fresh` だけを知っています**（立ち上がり直したあと）。
    /// `initialize` には `fresh` を渡し、知らない会期には 404 を素の文字で返します。
    fn restarted_server() -> (String, Seen) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("口を開けない");
        let port = listener.local_addr().unwrap().port();
        let seen: Seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = std::sync::Arc::clone(&seen);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let (mut length, mut session) = (0usize, None::<String>);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(rest) = lower.strip_prefix("content-length:") {
                        length = rest.trim().parse().unwrap_or(0);
                    }
                    if let Some(rest) = lower.strip_prefix("mcp-session-id:") {
                        session = Some(rest.trim().to_string());
                    }
                    if line == "\r\n" || line == "\n" {
                        break;
                    }
                }
                let mut body = vec![0u8; length];
                let _ = reader.read_exact(&mut body);
                let body: serde_json::Value =
                    serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
                let method = body["method"].as_str().unwrap_or("").to_string();
                record
                    .lock()
                    .unwrap()
                    .push((method.clone(), session.clone()));

                let reply = if method == "initialize" {
                    let payload = r#"{"jsonrpc":"2.0","id":0,"result":{}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nmcp-session-id: fresh\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                        payload.len()
                    )
                } else if session.as_deref() != Some("fresh") {
                    let text = "Not Found: Session not found";
                    format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                        text.len()
                    )
                } else if body.get("id").is_none() {
                    "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string()
                } else {
                    let payload = format!(
                        r#"{{"jsonrpc":"2.0","id":{},"result":{{"ok":true}}}}"#,
                        body["id"]
                    );
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                        payload.len()
                    )
                };
                let _ = stream.write_all(reply.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("http://127.0.0.1:{port}/mcp"), seen)
    }

    #[test]
    fn a_lost_session_is_set_up_again_and_the_call_goes_through() {
        let (url, seen) = restarted_server();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let answered = runtime.block_on(async {
            let client = reqwest::Client::builder().build().expect("client");
            // 立ち上がり直す前の会期と、そのとき相手が送った `initialize` を持っている状態。
            let session = std::sync::Mutex::new(Some("stale".to_string()));
            let hello = std::sync::Mutex::new(Some(
                r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#.to_string(),
            ));
            super::exchange(
                &client,
                &url,
                "test",
                &session,
                &hello,
                r#"{"jsonrpc":"2.0","id":7,"method":"tools/call"}"#.to_string(),
            )
            .await
        });

        let answered = answered.expect("**返事が 1 つも無い（相手は待ち続ける）**");
        let parsed: serde_json::Value =
            serde_json::from_str(&answered[0]).expect("**返事が JSON-RPC になっていない**");
        assert_eq!(parsed["id"], 7, "別の呼びの返事になっている: {answered:?}");
        assert_eq!(
            parsed["result"]["ok"], true,
            "張り直したあとの返事ではない: {answered:?}"
        );

        let methods: Vec<String> = seen
            .lock()
            .unwrap()
            .iter()
            .map(|(m, _)| m.clone())
            .collect();
        assert_eq!(
            methods,
            [
                "tools/call",
                "initialize",
                "notifications/initialized",
                "tools/call"
            ],
            "張り直しの手順が違う"
        );
    }

    #[test]
    fn without_the_opening_handshake_it_still_answers_in_json() {
        // **何も返さない、を作らない。**張り直せないときも、その呼びには必ず答える。
        let (url, _) = restarted_server();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let answered = runtime.block_on(async {
            let client = reqwest::Client::builder().build().expect("client");
            let session = std::sync::Mutex::new(Some("stale".to_string()));
            let hello = std::sync::Mutex::new(None::<String>);
            super::exchange(
                &client,
                &url,
                "test",
                &session,
                &hello,
                r#"{"jsonrpc":"2.0","id":9,"method":"tools/call"}"#.to_string(),
            )
            .await
        });
        let answered = answered.expect("**返事が 1 つも無い（相手は待ち続ける）**");
        let parsed: serde_json::Value =
            serde_json::from_str(&answered[0]).expect("**返事が JSON-RPC になっていない**");
        assert_eq!(parsed["id"], 9);
        assert!(
            parsed.get("error").is_some(),
            "失敗を失敗として返していない: {answered:?}"
        );
    }
}
