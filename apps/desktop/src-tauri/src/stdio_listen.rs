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

/// 溜めておける上限。**区切りの来ない流れで、手元のメモリを使い切らない**（レビュー LOW-6）。
pub const MAX_HELD_BYTES: usize = 1024 * 1024;

/// 上限を超えた。**流れを切って開け直します。**
#[derive(Debug)]
pub struct Overflow;

/// SSE の流れを、**届いた切れ端から**イベントごとに組み直す。
///
/// 流れは好きな所で切れて届きます（イベントの途中・行の途中・**字の途中**）。
/// **バイトのまま溜め**、空行で終わったイベントだけを字に直して渡します ——
/// 切れ端ごとに字に直すと、日本語が切れ目で壊れます（レビュー M4）。
#[derive(Default)]
pub struct SseBuffer {
    held: Vec<u8>,
}

impl SseBuffer {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, Overflow> {
        self.held.extend_from_slice(chunk);
        let Some(end) = last_break(&self.held) else {
            if self.held.len() > MAX_HELD_BYTES {
                self.held.clear();
                return Err(Overflow);
            }
            return Ok(Vec::new());
        };
        let done: Vec<u8> = self.held.drain(..end).collect();
        let text = String::from_utf8_lossy(&done).replace("\r\n", "\n");
        Ok(crate::stdio_proxy::parse_sse(&text))
    }
}

/// 最後のイベントの区切り（空行）の**直後**の位置。`\n\n` と `\r\n\r\n` の両方を見ます。
fn last_break(bytes: &[u8]) -> Option<usize> {
    (1..bytes.len()).rev().find_map(|at| {
        let ends_line = bytes[at] == b'\n';
        let blank_before = bytes[at - 1] == b'\n'
            || (at >= 3 && bytes[at - 1] == b'\r' && bytes[at - 2] == b'\n')
            || (at >= 3 && &bytes[at - 3..at] == b"\r\n\r");
        (ends_line && blank_before).then_some(at + 1)
    })
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
            let Ok(lines) = buffer.push(&chunk) else {
                eprintln!(
                    "[sshboard] 本体からの知らせが区切られないまま大きすぎるので、開け直します"
                );
                break;
            };
            for line in lines {
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
    fn a_character_split_between_two_chunks_is_not_broken() {
        // **バイトの切れ目で字を壊さない**（レビュー M4・LOW-4）。日本語は 3 バイト。
        let event = format!(
            "data: {}\n\n",
            r#"{"jsonrpc":"2.0","method":"x","params":{"content":"人が答えました"}}"#
        );
        let bytes = event.as_bytes();
        let cut = event.find('人').expect("字") + 1;
        let mut buffer = SseBuffer::default();

        assert!(buffer.push(&bytes[..cut]).expect("溢れていない").is_empty());
        let out = buffer.push(&bytes[cut..]).expect("溢れていない");

        assert_eq!(out.len(), 1);
        assert!(out[0].contains("人が答えました"), "字が壊れた: {}", out[0]);
    }

    #[test]
    fn a_crlf_break_split_between_chunks_still_ends_the_event() {
        // `\r\n\r\n` の途中で切れても、区切りを見落とさない（レビュー LOW-5）。
        let mut buffer = SseBuffer::default();
        assert!(buffer
            .push(format!("data: {NOTICE}\r\n\r").as_bytes())
            .expect("ok")
            .is_empty());
        assert_eq!(buffer.push(b"\n").expect("ok"), vec![NOTICE.to_string()]);
    }

    #[test]
    fn a_stream_that_never_ends_an_event_is_cut_off() {
        // **終わらない流れで、手元のメモリを使い切らない**（レビュー LOW-6）。
        let mut buffer = SseBuffer::default();
        let huge = vec![b'x'; MAX_HELD_BYTES + 1];
        assert!(buffer.push(&huge).is_err(), "上限を超えても溜め続けた");
    }

    #[test]
    fn an_event_cut_in_the_middle_comes_out_whole_once_it_ends() {
        let mut buffer = SseBuffer::default();
        let (head, tail) = NOTICE.split_at(20);

        assert!(buffer
            .push(format!("data: {head}").as_bytes())
            .expect("ok")
            .is_empty());
        assert!(
            buffer
                .push(format!("{tail}\n").as_bytes())
                .expect("ok")
                .is_empty(),
            "終わる前に渡した"
        );
        assert_eq!(buffer.push(b"\n").expect("ok"), vec![NOTICE.to_string()]);
    }

    #[test]
    fn the_empty_retry_frame_is_not_passed_on() {
        // 繋いだ直後の `data: \nid: 0\nretry: 3000` は JSON-RPC ではない（stdio_proxy と同じ）。
        let mut buffer = SseBuffer::default();
        let out = buffer
            .push(format!("data: \nid: 0\nretry: 3000\n\ndata: {NOTICE}\n\n").as_bytes())
            .expect("ok");
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
