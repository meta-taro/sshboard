//! 帯 → 画面。
//!
//! **預けてから出す。**出したあとに預けると、画面からの ack が先に着いて取りこぼす。

use serde::Serialize;
use sshboard_band::{BandEvent, RecvError, Subscriber};
use tauri::{AppHandle, Emitter, Manager};

use crate::pending::PendingLines;

/// 画面が待ち受けるイベント名。
pub const BAND_EVENT: &str = "band://line";

/// 画面へ渡す 1 行。**接続先を入れないこと**（PRD §8）。
#[derive(Clone, Serialize)]
pub struct BandLinePayload {
    pub seq: u64,
    /// `[AI]` / `[Human]`。
    pub tag: &'static str,
    pub text: String,
    /// 行頭を揃えた表示用の 1 行。
    pub rendered: String,
}

/// 帯を購読して画面へ流し続ける。
pub fn spawn(app: AppHandle, mut subscriber: Subscriber) {
    tauri::async_runtime::spawn(async move {
        loop {
            match subscriber.recv().await {
                Ok(event) => forward(&app, event),
                // 取りこぼしたことを黙らない。帯は記録なので、欠けたなら欠けたと出す。
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("[sshboard] 帯の行を {missed} 本取りこぼしました");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn forward(app: &AppHandle, event: BandEvent) {
    let line = event.line();
    let seq = line.seq();
    let payload = BandLinePayload {
        seq,
        tag: line.actor().tag(),
        text: line.text().to_owned(),
        rendered: line.render(),
    };

    if let Err(error) = app.state::<PendingLines>().hold(event) {
        eprintln!("[sshboard] 帯の行を預けられません: {error}");
        return;
    }

    // 出せなかったときに ack しない。出していないものを「出した」と返さない（D16）。
    if let Err(error) = app.emit(BAND_EVENT, payload) {
        eprintln!("[sshboard] 帯の行を画面へ渡せません: seq={seq} {error}");
    }
}
