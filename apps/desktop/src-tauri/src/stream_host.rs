//! 追尾している出力を画面へ流す（Issue 005 / D60）。
//!
//! **MCP と同じものを共有する。**GUI 用にもう 1 本流すのは
//! 「裏で見えないセッションを張らない」（PRD §4-1）に反する。
//!
//! **接続ごとに 1 本**流れます（D60・2026-10-02）。端末が接続ごとに
//! 持てるようになったので、**口を 1 本にしたままだと 2 台の出力が混ざります。**
//! 混ざったものは人も AI も読めません。**どの接続のものかを必ず添えます。**

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use sshboard_engine::Engine;
use sshboard_stream::{OutputStream, RecvError};
use tauri::{AppHandle, Emitter};

/// 画面が待ち受けるイベント名。**ANSI を落とさずに渡す。**
pub const STREAM_EVENT: &str = "stream://raw";

/// 出力の塊 1 つ。**どこのものかを添えます**（D60）。
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    /// どの接続か。**識別子だけ**（ホスト名は入りません・PRD §8）。
    ///
    /// `None` は**まだ 1 本も繋いでいない間**の出力です。
    pub connection: Option<String>,
    /// **どの端末のものか**（2026-10-06・分割）。
    ///
    /// `None` は**端末ではないもの** —— 用途別ツールの出力や `tail -f` です。
    /// 画面は、これが入っていればその面へだけ書きます ——
    /// **同じサーバに 2 枚開いた日に、両方へ同じものを映さないため。**
    pub console: Option<u64>,
    pub chunk: Vec<u8>,
}

/// 繋ぐ前の出力を画面へ流す。**繋ぐ前にも出るものがあります。**
pub fn spawn_bridge(app: AppHandle, stream: Arc<OutputStream>) {
    pump(app, None, None, stream);
}

/// **開いている接続ぜんぶの出力**を画面へ流し続ける（D60）。
///
/// 開いたものが増えたら、その接続の口へも繋ぎます。
/// **繋ぎ忘れると、その接続の画面だけが無言になります** ——
/// 「開いたのに映らない」は Issue #10 で実際に踏んだ形です。
pub fn spawn_connection_bridges(app: AppHandle, engine: Arc<Engine>) {
    tauri::async_runtime::spawn(async move {
        let mut watching = engine.subscribe();
        // **同じ接続へ 2 本繋がない。**繋ぐと 1 文字が 2 回出ます。
        let mut wired: HashMap<String, ()> = HashMap::new();
        loop {
            let ids: Vec<String> = watching
                .borrow_and_update()
                .iter()
                .map(|open| open.id.clone())
                .collect();
            for id in ids {
                if wired.contains_key(&id) {
                    continue;
                }
                let stream = engine.stream_for(&id).await;
                pump(app.clone(), Some(id.clone()), None, stream);
                wired.insert(id, ());
            }
            // **切れた接続は忘れる。**繋ぎ直したら、また繋げるようにします
            // （engine は切断で口を捨てるので、同じ口は残っていません）。
            let still: Vec<String> = watching
                .borrow()
                .iter()
                .map(|open| open.id.clone())
                .collect();
            wired.retain(|id, _| still.contains(id));

            if watching.changed().await.is_err() {
                break;
            }
        }
    });
}

/// **端末ごとの出力**を画面へ流し続ける（2026-10-06・分割）。
///
/// 接続ごとの口とは別です。**同じサーバに 2 枚開くと、
/// 接続ごとの口には両方の出力が混ざって流れます** ——
/// 左右の面を見比べる人には、それでは使えません。
pub fn spawn_console_bridges(app: AppHandle, engine: Arc<Engine>) {
    tauri::async_runtime::spawn(async move {
        let mut watching = engine.subscribe_consoles();
        // **同じ端末へ 2 本繋がない。**繋ぐと 1 文字が 2 回出ます。
        // 番号は使い回されないので、**消えたものを忘れる必要はありません** ——
        // 忘れないほうが安全です（閉じた番号が再び来ることはない）。
        let mut wired: HashMap<u64, ()> = HashMap::new();
        loop {
            let open: Vec<(u64, String)> = watching
                .borrow_and_update()
                .iter()
                .map(|facts| (facts.id, facts.connection.clone()))
                .collect();
            for (id, connection) in open {
                if wired.contains_key(&id) {
                    continue;
                }
                if let Some(stream) = engine.console_stream(id).await {
                    pump(app.clone(), Some(connection), Some(id), stream);
                    wired.insert(id, ());
                }
            }
            if watching.changed().await.is_err() {
                break;
            }
        }
    });
}

fn pump(
    app: AppHandle,
    connection: Option<String>,
    console: Option<u64>,
    stream: Arc<OutputStream>,
) {
    let mut raw = stream.subscribe_raw();
    tauri::async_runtime::spawn(async move {
        loop {
            match raw.recv().await {
                Ok(chunk) => {
                    let payload = StreamChunk {
                        connection: connection.clone(),
                        console,
                        chunk,
                    };
                    if let Err(error) = app.emit(STREAM_EVENT, payload) {
                        eprintln!("[sshboard] 出力を画面へ渡せません: {error}");
                    }
                }
                // 取りこぼしたことを黙らない。
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("[sshboard] 出力を {missed} 個取りこぼしました");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}
