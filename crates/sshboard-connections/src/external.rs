//! 外から書き換えられた接続一覧を拾う。
//!
//! **なぜ要るか**: [`ConnectionsWatch`](crate::ConnectionsWatch) が鳴るのは
//! **この中で書き換えたとき**だけだった。画面が保存した・MCP が書き換えた、は鳴る。
//! **人がエディタで書き換えた、は鳴らない。**
//! 「作ってあるが繋いでいない」ではなく、**外を見る口が最初から無かった**（D59）。
//!
//! **watcher の crate を足しません。**2 秒ごとに読み直して、姿が変わったかだけを見ます。
//! 一覧は人が手で書ける大きさで、OS ごとの watcher の癖
//! （macOS の取りこぼし・Windows のファイルロック）を持ち込む理由が無い。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::store::Connections;
use crate::watch::ConnectionsWatch;

/// 見に行く間隔。
const LOOK_EVERY: Duration = Duration::from_secs(2);

/// ファイルの姿。
///
/// **中身は持ちません。**持つと、見張りの側に接続先が residues として残る（PRD §8）。
/// 大きさと mtime ではなく中身から作るのは、**`id` を 1 文字だけ直した書き換え**
/// （長さが変わらず、mtime の刻みに入ることもある）を取り落とさないため。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FileStamp(Option<u64>);

impl FileStamp {
    /// いま置かれているものの姿を取る。**無いことも 1 つの姿。**
    pub fn of(path: &Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self(None);
        };
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self(Some(hasher.finish()))
    }
}

/// 1 回見た結果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look {
    /// 変わっていない。
    Same,
    /// 変わって、読める。**流す。**
    Changed,
    /// 変わったが、接続一覧として読めない。**流さない。**
    Unreadable,
}

/// 1 回だけ見る。
///
/// 印は呼び手が持ちます。**同じ壊れたファイルで 2 秒ごとに騒がない**ため、
/// 読めなかったときも印は進めます。
pub fn look_once(path: &Path, seen: &mut FileStamp) -> Look {
    let now = FileStamp::of(path);
    if now == *seen {
        return Look::Same;
    }
    *seen = now;

    // **書き途中を拾わない。**エディタが保存している最中のものを読むと、
    // 画面に「読めません」の帯が一瞬出る。書き途中は「新しい状態」ではない。
    if Connections::load_or_empty(path).is_err() {
        return Look::Unreadable;
    }
    Look::Changed
}

/// 外からの書き換えを拾って [`ConnectionsWatch`](crate::ConnectionsWatch) へ流し続ける。
///
/// **始めた時点の姿を印にします。**そうしないと、立ち上がった直後に 1 回鳴る。
///
/// 呼び手が spawn します（この crate は runtime を前提にしません）。
pub async fn look_for_external_changes(path: PathBuf, watch: Arc<ConnectionsWatch>) {
    let mut seen = FileStamp::of(&path);
    let mut ticks = tokio::time::interval(LOOK_EVERY);
    ticks.tick().await; // 1 回目はすぐ返る

    loop {
        ticks.tick().await;
        match look_once(&path, &mut seen) {
            Look::Same => {}
            Look::Changed => watch.notify(),
            // **置き場所は書きません。**手元の path に OS の利用者名が入る。
            Look::Unreadable => {
                eprintln!("[sshboard] 接続一覧が、読めない形に変わりました");
            }
        }
    }
}
