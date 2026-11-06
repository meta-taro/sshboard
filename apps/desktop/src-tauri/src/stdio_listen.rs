//! **本体から来る知らせを、受け続ける**（Issue #29・2026-11-06）。
//!
//! 中継は「stdin の 1 行を POST し、返事を stdout へ」だけをしていました。
//! **呼んでいないのに本体から来るもの**（人が答えた知らせなど）は、Streamable HTTP では
//! 別に開けた GET の流れに乗ります。**その流れを誰も開けていなかったので、1 通も届いていませんでした。**
//!
//! ここは会期の番号が決まったら GET を開け、届いた JSON-RPC を 1 行ずつ渡します。
//! 切れたら少し待って開け直します（本体が立ち上がり直したときは、新しい番号で）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 開け直すまでの間。**本体が落ちている間に、張り付いて叩かない。**
const RETRY_AFTER: Duration = Duration::from_secs(1);

/// SSE の流れを、**届いた切れ端から**イベントごとに組み直す。
///
/// 流れは好きな所で切れて届きます（イベントの途中・行の途中）。
/// **空行で終わったイベントだけを渡し、残りは次の切れ端と繋ぎます。**
#[derive(Default)]
pub struct SseBuffer {
    held: String,
}

impl SseBuffer {
    pub fn push(&mut self, chunk: &str) -> Vec<String> {
        self.held.push_str(&chunk.replace("\r\n", "\n"));
        let Some(end) = self.held.rfind("\n\n") else {
            return Vec::new();
        };
        let done: String = self.held.drain(..end + 2).collect();
        crate::stdio_proxy::parse_sse(&done)
    }
}

/// 受け続ける。**戻りません**（中継が終わるときに一緒に落ちます）。
///
/// 本体がこの流れを持たない（405）ときだけ、諦めて戻ります。
pub async fn listen(
    client: Arc<reqwest::Client>,
    url: Arc<String>,
    token: Arc<String>,
    session: Arc<Mutex<Option<String>>>,
    emit: impl Fn(String),
) {
    loop {
        let held = session.lock().ok().and_then(|held| held.clone());
        let Some(id) = held else {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        };
        let opened = client
            .get(url.as_str())
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .header("mcp-session-id", id)
            .send()
            .await;
        let mut response = match opened {
            Ok(response) if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED => {
                eprintln!("[sshboard] 本体が知らせの流れを持っていません（呼びの返事は届きます）");
                return;
            }
            Ok(response) if response.status().is_success() => response,
            _ => {
                tokio::time::sleep(RETRY_AFTER).await;
                continue;
            }
        };
        let mut buffer = SseBuffer::default();
        while let Ok(Some(chunk)) = response.chunk().await {
            for line in buffer.push(&String::from_utf8_lossy(&chunk)) {
                emit(line);
            }
        }
        tokio::time::sleep(RETRY_AFTER).await;
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use super::*;

    const NOTICE: &str = r#"{"jsonrpc":"2.0","method":"notifications/claude/channel","params":{"content":"answered"}}"#;

    #[test]
    fn an_event_cut_in_the_middle_comes_out_whole_once_it_ends() {
        let mut buffer = SseBuffer::default();
        let (head, tail) = NOTICE.split_at(20);

        assert!(buffer.push(&format!("data: {head}")).is_empty());
        assert!(
            buffer.push(&format!("{tail}\n")).is_empty(),
            "終わる前に渡した"
        );
        assert_eq!(buffer.push("\n"), vec![NOTICE.to_string()]);
    }

    #[test]
    fn the_empty_retry_frame_is_not_passed_on() {
        // 繋いだ直後の `data: \nid: 0\nretry: 3000` は JSON-RPC ではない（stdio_proxy と同じ）。
        let mut buffer = SseBuffer::default();
        let out = buffer.push(&format!("data: \nid: 0\nretry: 3000\n\ndata: {NOTICE}\n\n"));
        assert_eq!(out, vec![NOTICE.to_string()]);
    }

    /// GET が来たら、会期の番号を控えて 1 通だけ流す相手。
    fn notifying_server() -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("口を開けない");
        let url = format!("http://{}/mcp", listener.local_addr().expect("番地"));
        let (seen_tx, seen_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().expect("複製"));
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            let _ = seen_tx.send(head);
            let mut stream = stream;
            let body = format!("data: {NOTICE}\n\n");
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
        });
        (url, seen_rx)
    }

    #[tokio::test]
    async fn a_notice_from_the_server_is_passed_on_with_the_session_it_belongs_to() {
        let (url, seen) = notifying_server();
        let session = Arc::new(Mutex::new(Some("s-123".to_string())));
        let got = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&got);

        let task = tokio::spawn(listen(
            Arc::new(reqwest::Client::new()),
            Arc::new(url),
            Arc::new("t".to_string()),
            session,
            move |line| sink.lock().expect("錠").push(line),
        ));
        for _ in 0..50 {
            if !got.lock().expect("錠").is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        task.abort();

        assert_eq!(*got.lock().expect("錠"), vec![NOTICE.to_string()]);
        let head = seen
            .recv_timeout(Duration::from_secs(2))
            .expect("GET が来ていない")
            .to_lowercase();
        assert!(head.starts_with("get "), "{head}");
        assert!(
            head.contains("mcp-session-id: s-123"),
            "会期の番号を載せていない: {head}"
        );
        assert!(head.contains("accept: text/event-stream"), "{head}");
    }
}
