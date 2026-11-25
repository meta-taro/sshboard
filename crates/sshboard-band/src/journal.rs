//! 帯の控え（Phase 2「監査ログの永続化」・2026-11-25）。
//!
//! ## なぜ要るか
//!
//! 帯は画面の中にしか無く、**アプリを閉じると消えていました。**
//! 「昨日 AI は何をしたか」を後から人が確かめる手段が、書き出しを押し忘れた日には無い。
//! 人と AI が同じ 1 本の SSH を使う道具で、それは記録として欠けています。
//!
//! ## 何を残すか
//!
//! **帯に出た行だけ**です（誰が・何を）。端末の出力や打鍵の中身は残しません ——
//! 帯の行にはパスワードが乗らない作りで、そこから外へ広げません。
//! ただし**接続の識別子とサーバー側のパスは入ります。**置き場所はこの PC の中だけで、
//! 本人だけが読める権限で書きます。
//!
//! ## どれだけ残すか
//!
//! 1 日 1 本（UTC の日付）。**`keep_days` を過ぎた日は消します** ——
//! 際限なく残すと、残す場所を増やさないという方針（端末の出力はディスクに書かない）を
//! 帯の側から崩すことになります。

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::line::BandLine;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const PREFIX: &str = "band-";
const SUFFIX: &str = ".jsonl";

/// 控えの 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    /// いつ（UNIX 時刻・ミリ秒）。
    pub at_ms: u64,
    /// 帯の中での通し番号。**アプリを立ち上げ直すと 0 から**数え直します。
    pub seq: u64,
    /// `[AI]` / `[Human]`。
    pub tag: String,
    pub text: String,
}

/// 帯の控えを置く場所。
#[derive(Debug, Clone)]
pub struct Journal {
    dir: PathBuf,
    keep_days: u64,
}

impl Journal {
    pub fn new(dir: impl Into<PathBuf>, keep_days: u64) -> Self {
        Self {
            dir: dir.into(),
            keep_days,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 1 行足す。
    pub fn append(&self, at_ms: u64, line: &BandLine) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let entry = JournalEntry {
            at_ms,
            seq: line.seq(),
            tag: line.actor().tag().to_owned(),
            text: line.text().to_owned(),
        };
        let mut json = serde_json::to_string(&entry).map_err(io::Error::other)?;
        json.push('\n');
        let mut file = open_private_append(&self.path_for(&day_of(at_ms)))?;
        file.write_all(json.as_bytes())
    }

    /// 控えのある日。**新しい順。**
    pub fn days(&self) -> io::Result<Vec<String>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let mut days: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| day_from_file_name(&entry.file_name().to_string_lossy()))
            .collect();
        days.sort_unstable_by(|a, b| b.cmp(a));
        Ok(days)
    }

    /// その日の控えを、書いた順に読む。
    ///
    /// **日付の形でないものは断ります** —— 画面から渡る文字をそのままパスに
    /// 繋ぐと、`../connections` のような名前で別のファイルを開けてしまいます。
    pub fn read_day(&self, day: &str) -> io::Result<Vec<JournalEntry>> {
        if !is_day(day) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("日付の形ではありません: {day}"),
            ));
        }
        let file = fs::File::open(self.path_for(day))?;
        let mut entries = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // **壊れた 1 行で全部を読めなくしない。**書いている途中で落ちると
            // 最後の行が欠けます。欠けた行は飛ばし、残りは見せます。
            if let Ok(entry) = serde_json::from_str::<JournalEntry>(&line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// `keep_days` を過ぎた日を消す。**控えでないファイルには触りません。**
    pub fn prune(&self, now_ms: u64) -> io::Result<()> {
        let oldest_kept = day_of(now_ms.saturating_sub(self.keep_days * DAY_MS));
        for day in self.days()? {
            if day < oldest_kept {
                fs::remove_file(self.path_for(&day))?;
            }
        }
        Ok(())
    }

    fn path_for(&self, day: &str) -> PathBuf {
        self.dir.join(format!("{PREFIX}{day}{SUFFIX}"))
    }
}

fn day_from_file_name(name: &str) -> Option<String> {
    let day = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    is_day(day).then(|| day.to_owned())
}

/// `YYYY-MM-DD` か。
fn is_day(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// UNIX 時刻（ミリ秒）を UTC の `YYYY-MM-DD` にする。
///
/// 日付のための依存を足さないため、暦の計算をここに置きます
/// （Howard Hinnant の `civil_from_days`）。
fn day_of(at_ms: u64) -> String {
    let days = (at_ms / DAY_MS) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(unix)]
fn open_private_append(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;
    // `mode()` は作るときにしか効かないので、既にあるファイルは締め直す。
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

#[cfg(not(unix))]
fn open_private_append(path: &Path) -> io::Result<fs::File> {
    // Windows は利用者ごとの AppData の下に置くことで、ほかの利用者から隔てます。
    OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_counted_on_the_utc_calendar() {
        assert_eq!(day_of(0), "1970-01-01");
        assert_eq!(day_of(951_782_400_000), "2000-02-29");
        assert_eq!(day_of(1_795_564_800_000), "2026-11-25");
        assert_eq!(day_of(1_795_564_800_000 - 1), "2026-11-24");
    }
}
