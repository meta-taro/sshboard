//! 断った事実の追記（D3 追記・2026-08-27）。
//!
//! **許可リストを推測で埋めない代わりに、足りなかったものを機械が残します。**
//! 人はこのファイルを見て、実務で本当に要ったものだけを足せます。
//!
//! 1 行 3 列（タブ区切り）。`<TAB>` は実際にはタブ 1 文字です:
//!
//! ```text
//! 2026-09-01T04:12:33Z<TAB>ai<TAB>uptime
//! ```
//!
//! **接続先・利用者名・引数は 1 つも入りません**（PRD §8）。
//! 入るのは「いつ・どちらが・どの識別子を求めたか」だけです。

use std::io::Write;
use std::path::{Path, PathBuf};

use sshboard_band::Actor;

/// 1 件に残す識別子の長さの上限。
/// **長い文字列を延々と流し込まれて、記録が読めなくなるのを防ぐ。**
pub const MAX_ID_CHARS: usize = 200;

/// 記録が読めなくなる文字を置き換える印。
const REPLACEMENT: char = '·';

/// 断った事実の置き場所。**追記だけします。読み書きの管理はしません。**
pub struct Refusals {
    path: PathBuf,
}

impl Refusals {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 1 件残す。**前の行は消しません。**
    ///
    /// 失敗を握り潰さず返します。呼ぶ側は**記録できなくてもコマンドは断る**こと。
    /// 「記録できないから通す」は、いちばんやってはいけない転び方です。
    pub fn record(&self, actor: Actor, requested_id: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let line = format!(
            "{}\t{}\t{}\n",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            side(actor),
            single_column(requested_id),
        );

        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        // 同じ機械の他の利用者に読ませない（接続一覧と揃える）。
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        options.open(&self.path)?.write_all(line.as_bytes())
    }
}

fn side(actor: Actor) -> &'static str {
    match actor {
        Actor::Ai => "ai",
        Actor::Human => "human",
    }
}

/// 1 列に収める。**呼ぶ側の文字列で行や列を割らせない。**
fn single_column(requested_id: &str) -> String {
    let mut cleaned: String = requested_id
        .chars()
        .take(MAX_ID_CHARS)
        .map(|c| {
            if c.is_control() || c == '\t' {
                REPLACEMENT
            } else {
                c
            }
        })
        .collect();

    // **切ったことを隠さない。**
    if requested_id.chars().count() > MAX_ID_CHARS {
        cleaned.push('…');
    }
    cleaned
}
