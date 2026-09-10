//! **この道具が何なのかを、AI へ名乗る。**
//!
//! 実機からの要望（2026-09-09）:
//!
//! > MCP にこのソフト概要みたいなのを AI エージェント向けに出力するやつ設置したいです。
//! > **バージョンごとになにが変わったかも返す**ように。
//!
//! ## なぜ焼き込むのか
//!
//! `CHANGELOG.md` を**バイナリへ埋め込みます**（`include_str!`）。
//!
//! ファイルとして読みに行くと、**配った版と中身がずれます** ——
//! 0.1.9 の実行ファイルが、手元に置かれた 0.1.11 の履歴を読んでしまう。
//! **走っている物が、自分について嘘をつかない**ことを優先しました。
//!
//! 走っている版が履歴に無ければ、テストが落ちます
//! （`the_running_version_is_in_the_history`）。
//!
//! ## 何を書くか
//!
//! 概要は**ここに直書き**します。設定にすると、人が書き換えられる代わりに
//! **書き換えられたことに誰も気づけません。**これは「AI に何を約束しているか」の
//! 文言なので、**コードと同じ扱いで読まれるべき**です。

/// 変更履歴。**このファイルの形式に依存しています**（`## <版> — <一行>`）。
pub const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// 版 1 つぶん。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// `0.1.11` のような番号。**`v` も `-alpha.N` も付きません。**
    pub version: String,
    /// 見出しの一行。**何のための版か。**
    pub headline: String,
    /// 中身（箇条書きのまま）。
    pub body: String,
}

/// 履歴を版ごとに切り出す。**新しい順。**
///
/// 見出しの形（`## <版> — <一行>`）が崩れると**1 件も返りません。**
/// 人は読めてしまうので、**テストで見張っています。**
pub fn releases(changelog: &str) -> Vec<Release> {
    let mut found = Vec::new();
    let mut current: Option<(String, String, Vec<&str>)> = None;

    for line in changelog.lines() {
        let Some(heading) = line.strip_prefix("## ") else {
            if let Some((_, _, body)) = current.as_mut() {
                body.push(line);
            }
            continue;
        };
        if let Some((version, headline, body)) = current.take() {
            found.push(finish(version, headline, &body));
        }
        // `0.1.11 — 一行` を割る。**全角のダッシュで区切っています。**
        let (version, headline) = match heading.split_once(" — ") {
            Some((version, headline)) => (version.trim(), headline.trim()),
            // 一行が無くても捨てません。**番号だけは拾えます。**
            None => (heading.trim(), ""),
        };
        current = Some((version.to_owned(), headline.to_owned(), Vec::new()));
    }
    if let Some((version, headline, body)) = current {
        found.push(finish(version, headline, &body));
    }
    found
}

fn finish(version: String, headline: String, body: &[&str]) -> Release {
    Release {
        version,
        headline,
        body: body.join("\n").trim().to_owned(),
    }
}

/// `since` より**あとに出た**ぶんだけ。**新しい順。**
///
/// - `None` … 全部
/// - 最新を渡された … **空**（「変わっていない」と「全部変わった」を混ぜない）
/// - 知らない版 … **`None`。**黙って空を返すと、
///   **打ち間違いが「変わっていません」に化けます**（product-baseline §8）
pub fn changes_since(changelog: &str, since: Option<&str>) -> Option<Vec<Release>> {
    let all = releases(changelog);
    let Some(since) = since else {
        return Some(all);
    };
    let at = all.iter().position(|one| one.version == since)?;
    Some(all.into_iter().take(at).collect())
}
