//! **エージェント専用カーソル**の、本体から画面への道（2026-11-12）。
//!
//! MCP の `screen` → ここ → 画面へ `agent://act`（番号つき）→ 画面がカーソルを動かす →
//! 画面が `agent_act_done` で答えを返す → 番号で待っている呼びへ渡す。
//!
//! **人の OS のマウスとキーボードは使いません。**動くのは画面の中に描いたカーソルだけです。
//! 何を探し、何を押してよいかを決めるのは画面の側（`agent-cursor.ts`）です。
//!
//! **答えが来なければ時間で切ります。**窓が隠れている・画面が固まっているときに、
//! 呼んだ席を黙って待たせ続けない（2026-11-11・席どうしが互いを止めない）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use sshboard_mcp::{ActOutcome, AgentAct};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::oneshot;

/// 画面へ指示を配るイベント。
pub const AGENT_ACT_EVENT: &str = "agent://act";

/// 画面の答えを待つ長さ。カーソルが動くのは 0.65 秒なので、十分に長く、席を待たせすぎない長さ。
const ANSWER_TIMEOUT: Duration = Duration::from_secs(10);

/// 画面へ渡す 1 件。
#[derive(Serialize, Clone)]
struct Delivered<'a> {
    id: u64,
    act: &'a AgentAct,
}

/// 答えを待っている呼び（番号ごと）。
pub struct AgentCursor {
    app: AppHandle,
    next: AtomicU64,
    waiting: Mutex<HashMap<u64, oneshot::Sender<ActOutcome>>>,
}

impl AgentCursor {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            next: AtomicU64::new(1),
            waiting: Mutex::new(HashMap::new()),
        })
    }

    /// 画面へ渡し、答えを待つ。**来なければ理由つきで断ります。**
    pub async fn act(&self, act: &AgentAct) -> Result<ActOutcome, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.waiting
            .lock()
            .map_err(|_| "答えの控えが壊れています".to_string())?
            .insert(id, tx);

        if let Err(error) = self.app.emit(AGENT_ACT_EVENT, Delivered { id, act }) {
            self.forget(id);
            return Err(format!("画面へ渡せません: {error}"));
        }

        match tokio::time::timeout(ANSWER_TIMEOUT, rx).await {
            Ok(Ok(outcome)) => Ok(outcome),
            _ => {
                self.forget(id);
                Err(
                    "sshboard's window did not answer - ask the person to bring it to the front \
                     and try again"
                        .to_string(),
                )
            }
        }
    }

    /// 画面の答えを、待っている呼びへ渡す。**知らない番号は捨てます**（時間切れの後に来た答え）。
    fn answer(&self, id: u64, outcome: ActOutcome) {
        let sender = self
            .waiting
            .lock()
            .ok()
            .and_then(|mut held| held.remove(&id));
        if let Some(sender) = sender {
            // 待っていた側が既に諦めていれば届かない。それは時間切れで返し済み。
            let _ = sender.send(outcome);
        }
    }

    fn forget(&self, id: u64) {
        if let Ok(mut held) = self.waiting.lock() {
            held.remove(&id);
        }
    }
}

/// 画面が、カーソルを動かした結果を返す口。
#[tauri::command]
pub fn agent_act_done(cursor: State<'_, Arc<AgentCursor>>, id: u64, outcome: ActOutcome) {
    cursor.answer(id, outcome);
}
