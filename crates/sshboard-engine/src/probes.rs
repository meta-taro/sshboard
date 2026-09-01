//! 用途別の読み取りツールが打つコマンド（PRD §「AI が呼べるもの」）。
//!
//! **ここが `run_command(cmd)` を作らずに済ませる方の半分です**（D3）。
//! AI はコマンドを組み立てません。**組み立てるのはここで、引数は必ず囲います。**
//!
//! ## 決めごと
//!
//! - **引数は例外なく [`quote`] を通す。**「安全そうな文字だけ素通し」を作らない
//! - **返ってこないコマンドを作らない。**`systemctl status` は既定でページャへ流すので、
//!   端末の無い `exec` では止まったまま返りません（`--no-pager` が要る）
//! - **落ちる先を用意する。**`ss` の無い機械は実在します。行き止まりにしない
//! - **書く語を混ぜない。**読み取りのツールです（`tests/probes.rs` が見張っています）

use sshboard_ssh::quote;

/// 一度に読む行数の上限。**丸ごとメモリに載るため。**
/// 実測でここに当たったら上げる（YAGNI）。
pub const MAX_LOG_LINES: u32 = 5_000;

/// 引数が足りない。**空のまま投げると、別のものが返ってきます。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingArgument {
    /// 何が空だったか。**値そのものは持ちません**（接続先が混ざりうるため）。
    pub what: &'static str,
}

impl std::fmt::Display for MissingArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}を指定してください", self.what)
    }
}

impl std::error::Error for MissingArgument {}

/// 空き容量。`-P` は**行が折り返さない**書式（長いマウント先で列がずれない）。
///
/// フラグは**まとめずに並べます。**このコマンドは帯に出て人が読むので、
/// `-hP` より `-h -P` の方が、何が効いているか見て分かります。
pub fn disk_usage() -> String {
    "df -h -P".to_string()
}

/// プロセス一覧。**自分の分だけでは障害調査に使えない**ので全部。
pub fn process_list() -> String {
    "ps aux".to_string()
}

/// listen しているポート。
///
/// `ss` が無い機械では `netstat` へ落ちます。**どちらも無ければ、そう返ります**
/// （握り潰さずに、シェルの言い分をそのまま返す）。
pub fn network_listen() -> String {
    "ss -ltnp 2>/dev/null || netstat -ltnp".to_string()
}

/// サービスの状態。
///
/// **`--no-pager` が命綱です。**無いと `systemctl` がページャを開こうとし、
/// 端末の無い `exec` では返ってきません。
pub fn service_status(name: &str) -> Result<String, MissingArgument> {
    if name.trim().is_empty() {
        // 空で投げると**全ユニットが返ります。**「押したのに違うものが出る」を作らない。
        return Err(MissingArgument {
            what: "サービス名"
        });
    }
    // `--` の後ろはオプションとして読まれない。**`-` で始まる名前を渡されても安全。**
    Ok(format!(
        "systemctl --no-pager --full status -- {}",
        quote(name)
    ))
}

/// 探して返す件数の上限。**切らないと返ってきません。**
pub const MAX_SEARCH_HITS: u32 = 500;

/// どこまで潜って探すか。**シンボリックリンクの輪に落ちないため**にも要る。
const SEARCH_DEPTH: u32 = 8;

/// 名前で探す。
///
/// **`/` から探されたら終わりません。**深さと件数の両方を切ります。
/// `head` で止めるので、`find` は途中で終わります（SIGPIPE）。
pub fn search_names(root: &str, pattern: &str, hits: u32) -> Result<String, MissingArgument> {
    if root.trim().is_empty() {
        return Err(MissingArgument {
            what: "探す場所"
        });
    }
    if pattern.trim().is_empty() {
        return Err(MissingArgument {
            what: "探す名前"
        });
    }
    let hits = clamp_hits(hits);
    // 読めないディレクトリの苦情は捨てる。**それは「見つからなかった」ではない**ので、
    // 結果の件数には効きません。
    Ok(format!(
        "find {} -maxdepth {SEARCH_DEPTH} -name {} -print 2>/dev/null | head -n {hits}",
        quote(root),
        quote(pattern)
    ))
}

/// 中身で探す。
///
/// `-I` でバイナリを飛ばします。**混ぜると端末が壊れます。**
/// `-n` は行番号で、これが無いと見つけたあとに人が辿れません。
pub fn search_content(root: &str, pattern: &str, hits: u32) -> Result<String, MissingArgument> {
    if root.trim().is_empty() {
        return Err(MissingArgument {
            what: "探す場所"
        });
    }
    if pattern.trim().is_empty() {
        return Err(MissingArgument { what: "探す語" });
    }
    let hits = clamp_hits(hits);
    // `--` を置く。**`-e` で始まる語がオプションとして読まれない。**
    // フラグは `df` と同じくまとめない。**帯に出て人が読む**ので、
    // `-rnI` より `-r -n -I` の方が、何が効いているか見て分かる。
    Ok(format!(
        "grep -r -n -I -- {} {} 2>/dev/null | head -n {hits}",
        quote(pattern),
        quote(root)
    ))
}

/// 何が入っていて、どの版か。
///
/// **入っていないことは異常ではありません。**`command -v` で在るものだけ尋ねます。
/// 無いものを走らせて「エラー」を返すと、AI は原因を探しに行って時間を溶かします。
///
/// `java` のように**版を標準エラーへ出す**ものがあるので `2>&1` で拾います。
pub fn runtime_versions() -> String {
    // 何の上で動いているかは、版と同じくらい効く。
    let mut command = String::from(
        ". /etc/os-release 2>/dev/null; printf 'os: %s\\n' \"${PRETTY_NAME:-unknown}\"; ",
    );
    command.push_str("for c in ");
    command.push_str(RUNTIMES.join(" ").as_str());
    command.push_str(
        "; do if command -v \"$c\" >/dev/null 2>&1; then \
         printf '%s: %s\\n' \"$c\" \"$(\"$c\" --version 2>&1 | head -n 1)\"; fi; done",
    );
    command
}

/// 版を尋ねる相手。**`--version` で答えるものだけ**を並べます。
/// `nginx` / `httpd` は `-v` なので入れません（**苦情を版として返さない**）。
const RUNTIMES: &[&str] = &[
    "python3", "node", "npm", "php", "ruby", "perl", "java", "go", "rustc", "psql", "mysql",
];

fn clamp_hits(hits: u32) -> u32 {
    hits.clamp(1, MAX_SEARCH_HITS)
}

/// ログの末尾。**追いかけません**（追うのは `follow`）。
pub fn read_log(path: &str, lines: u32) -> Result<String, MissingArgument> {
    if path.trim().is_empty() {
        return Err(MissingArgument { what: "パス" });
    }
    // 0 行は意味が無く、青天井は丸ごとメモリに載る。**両端で抑える。**
    let lines = lines.clamp(1, MAX_LOG_LINES);
    Ok(format!("tail -n {lines} -- {}", quote(path)))
}
