//! **帯の控え**（Phase 2「監査ログの永続化」・2026-11-25）。
//!
//! 帯は画面の中にしか無く、アプリを閉じると消えていました。
//! ここで帯をもう 1 本購読し、**この PC の中へ 1 日 1 本で書き足します。**
//! 画面からは［過去の帯］で日を選んで読み戻せます。
//!
//! 置き場所は `connections.toml` と同じフォルダの下の `band/`。
//! **14 日を過ぎた日は、立ち上げたときと日が変わったときに消します。**

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use sshboard_band::{Journal, JournalEntry, RecvError, Subscriber};
use tauri::State;

/// どれだけ残すか。**2 週間あれば「先週 AI は何をしたか」に答えられます。**
/// それより長く残すと、手元に接続の識別子とサーバー側のパスが溜まり続けます。
pub const KEEP_DAYS: u64 = 14;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// 既定の置き場所。`connections.toml` の隣。
pub fn default_dir() -> Option<PathBuf> {
    let connections = sshboard_connections::default_path().ok()?;
    Some(connections.parent()?.join("band"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 帯を購読して書き足し続ける。
///
/// **書いてから受け取りを返します。**返さないと、AI の呼び出しが
/// 「画面へ届いたか」を待つ所で、この購読の分だけ期限切れまで待たされます。
/// 書けなかったときも返します —— **控えが書けないことで、人の画面や AI を止めない。**
pub fn spawn_writer(journal: Journal, mut subscriber: Subscriber) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = journal.prune(now_ms()) {
            eprintln!("[sshboard] 帯の控えの古い日を消せません: {error}");
        }
        let mut last_pruned_day = now_ms() / DAY_MS;
        loop {
            match subscriber.recv().await {
                Ok(event) => {
                    let at = now_ms();
                    if let Err(error) = journal.append(at, event.line()) {
                        eprintln!("[sshboard] 帯の控えを書けません: {error}");
                    }
                    event.ack();
                    // **日が変わったら古い日を消す。**立ち上げっぱなしの PC で溜めないため。
                    if at / DAY_MS != last_pruned_day {
                        last_pruned_day = at / DAY_MS;
                        if let Err(error) = journal.prune(at) {
                            eprintln!("[sshboard] 帯の控えの古い日を消せません: {error}");
                        }
                    }
                }
                // 取りこぼしたことを黙らない。控えは記録なので、欠けたなら欠けたと出す。
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("[sshboard] 帯の控えが {missed} 行を取りこぼしました");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// 控えのある日（新しい順）と、置き場所。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalOverview {
    /// **絶対パスで言います**（探させない）。
    pub dir: String,
    pub days: Vec<String>,
    pub keep_days: u64,
}

#[tauri::command]
pub fn journal_days(journal: State<'_, Journal>) -> Result<JournalOverview, String> {
    Ok(JournalOverview {
        dir: journal.dir().display().to_string(),
        days: journal.days().map_err(|error| error.to_string())?,
        keep_days: KEEP_DAYS,
    })
}

/// その日の控え。**日付の形でないものは断ります**（別のファイルを開かせない）。
#[tauri::command]
pub fn journal_read(journal: State<'_, Journal>, day: String) -> Result<Vec<JournalEntry>, String> {
    journal.read_day(&day).map_err(|error| error.to_string())
}
