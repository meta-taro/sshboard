//! `~/.ssh/config` を読んで、登録の候補にする（UI 案・4 枚目）。
//!
//! ## AI はここを通れません
//!
//! **読むのは人の操作に限ります。**この解析器を呼ぶ口は画面にしか置きません ——
//! `~/.ssh/config` は**接続先そのものの一覧**で、
//! ホスト名・利用者名・鍵の置き場が並んでいます（禁止事項 4）。
//!
//! AI が読めると、**人が繋いでいない所まで全部見える**ことになります。
//! それは `list_connections` が「識別子だけを返す」と決めた意味を薄めます（D11）。
//!
//! ## 書きません
//!
//! **`~/.ssh/config` には 1 バイトも書きません。**読むだけです。
//! あのファイルは ssh 本体と他の道具が共有していて、
//! **こちらの書式の思い込みで壊すと、人は ssh そのものを失います。**
//!
//! ## 鍵の中身も読みません
//!
//! `IdentityFile` は**場所だけ**を控えます（D11 —— 鍵は ssh-agent と
//! OS の資格情報へ委ねる）。

use std::path::Path;

/// 読み取れた 1 件。**そのままでは登録しません** —— 人が確かめて選びます。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// `Host` に書かれていた名前。**そのまま識別子の候補**にします。
    pub id: String,
    /// `HostName`。無ければ `Host` をそのまま使います（ssh と同じ扱い）。
    pub host: String,
    /// `Port`。**無ければ 22** とは決めません —— `None` で返し、
    /// 既定を当てるのは呼ぶ側です（ここで埋めると「書いてあった」と区別できない）。
    pub port: Option<u16>,
    /// `User`。
    pub user: Option<String>,
    /// `IdentityFile`。**場所だけ**（中身は読みません）。
    pub key_path: Option<String>,
}

/// 読めなかった理由。
#[derive(Debug)]
pub enum SshConfigError {
    /// 置いていない。**異常ではありません**（使っていないだけ）。
    Missing,
    /// 在るのに読めない（権限など）。
    Unreadable(String),
}

impl std::fmt::Display for SshConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshConfigError::Missing => write!(
                f,
                "~/.ssh/config が見つかりません。**使っていないだけかもしれません** —— \
                 接続は手で登録できます"
            ),
            SshConfigError::Unreadable(detail) => write!(
                f,
                "~/.ssh/config を読めません: {detail}。\
                 **読めない一覧を空として扱いません** —— 0 件と「読めない」は別です"
            ),
        }
    }
}

impl std::error::Error for SshConfigError {}

/// `Host` で始まる塊を拾う。
///
/// **ssh の全書式は解きません。**`Include`・`Match`・トークン展開
/// （`%h` など）は扱わず、**素直に書かれた `Host` だけ**を候補にします。
///
/// 中途半端に解いて「取り込んだのに繋がらない」を作るより、
/// **拾えたものだけを出して、残りは人が手で足す**方が読めます。
pub fn parse(input: &str) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = Vec::new();
    let mut current: Option<Candidate> = None;

    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // `Key value` と `Key=value` の両方。ssh がどちらも受けます。
        let (key, value) = match line.split_once([' ', '\t', '=']) {
            Some((key, value)) => (key.trim(), value.trim().trim_start_matches('=').trim()),
            None => continue,
        };
        if value.is_empty() {
            continue;
        }
        let lowered = key.to_ascii_lowercase();

        if lowered == "host" {
            if let Some(done) = current.take() {
                found.push(done);
            }
            // **`*` や `?` を含むものは候補にしません。**
            // あれは「全部に当てる既定」で、**繋ぐ先ではありません。**
            // 取り込むと「`*` という名前のサーバー」が一覧に出ます。
            //
            // **並べて書かれた複数の名前も見送ります**（`Host a b`）——
            // どれを登録すべきか決められないので、人に任せます。
            if value.contains('*') || value.contains('?') || value.split_whitespace().count() != 1 {
                continue;
            }
            current = Some(Candidate {
                id: value.to_owned(),
                host: value.to_owned(),
                port: None,
                user: None,
                key_path: None,
            });
            continue;
        }

        let Some(entry) = current.as_mut() else {
            // `Host` より前に書かれた既定。**全部に当たる設定**なので、
            // 候補としては扱いません。
            continue;
        };

        match lowered.as_str() {
            "hostname" => entry.host = value.to_owned(),
            "port" => entry.port = value.parse().ok(),
            "user" => entry.user = Some(value.to_owned()),
            // **最初の 1 本だけ。**複数書けますが、どれを使うかは ssh が決めます。
            "identityfile" if entry.key_path.is_none() => {
                entry.key_path = Some(value.to_owned());
            }
            _ => {}
        }
    }

    if let Some(done) = current.take() {
        found.push(done);
    }
    found
}

/// 読む。**書きません。**
pub fn read(path: &Path) -> Result<Vec<Candidate>, SshConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(SshConfigError::Missing),
        Err(error) => Err(SshConfigError::Unreadable(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_a_plain_host() {
        let found = parse(
            "Host web\n  HostName web.example.test\n  User ops\n  Port 2222\n  IdentityFile ~/.ssh/id_ed25519\n",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "web");
        assert_eq!(found[0].host, "web.example.test");
        assert_eq!(found[0].user.as_deref(), Some("ops"));
        assert_eq!(found[0].port, Some(2222));
        assert_eq!(found[0].key_path.as_deref(), Some("~/.ssh/id_ed25519"));
    }

    #[test]
    fn a_host_without_a_hostname_uses_its_own_name() {
        // ssh と同じ扱い。**埋めないと「繋ぎ先が無い」候補が出ます。**
        let found = parse("Host db.example.test\n  User ops\n");
        assert_eq!(found[0].host, "db.example.test");
    }

    #[test]
    fn it_does_not_invent_a_port() {
        // **`None` で返す。**ここで 22 を埋めると、
        // 「書いてあった 22」と「書いていない」を区別できません。
        let found = parse("Host web\n  HostName web.example.test\n");
        assert_eq!(found[0].port, None);
    }

    #[test]
    fn it_skips_the_catch_all() {
        // **`Host *` は「全部に当てる既定」で、繋ぐ先ではありません。**
        // 取り込むと「`*` という名前のサーバー」が一覧に出ます。
        let found =
            parse("Host *\n  ServerAliveInterval 60\n\nHost web\n  HostName web.example.test\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "web");
    }

    #[test]
    fn it_skips_a_pattern_and_a_list() {
        // `?` を含むものと、並べて書かれたものは**人に任せます。**
        let found = parse("Host web?\n  User ops\n\nHost a b\n  User ops\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn it_ignores_defaults_written_before_any_host() {
        // `Host` より前の設定は**全部に当たる**ので、候補にしません。
        let found = parse("User ops\nPort 2222\n\nHost web\n  HostName web.example.test\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].user, None);
        assert_eq!(found[0].port, None);
    }

    #[test]
    fn it_accepts_the_equals_form() {
        // ssh はどちらも受けます。
        let found = parse("Host=web\n  HostName=web.example.test\n  Port=2222\n");
        assert_eq!(found[0].id, "web");
        assert_eq!(found[0].host, "web.example.test");
        assert_eq!(found[0].port, Some(2222));
    }

    #[test]
    fn it_takes_only_the_first_key() {
        // 複数書けますが、どれを使うかは ssh が決めます。
        let found = parse("Host web\n  IdentityFile ~/.ssh/a\n  IdentityFile ~/.ssh/b\n");
        assert_eq!(found[0].key_path.as_deref(), Some("~/.ssh/a"));
    }

    #[test]
    fn it_ignores_comments_and_blank_lines() {
        let found = parse("# さいしょ\n\nHost web\n  # なか\n  HostName web.example.test\n\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].host, "web.example.test");
    }

    #[test]
    fn a_bad_port_does_not_throw_the_host_away() {
        // **1 行読めなくても候補を捨てません。**
        // 捨てると、人は「なぜこのサーバーだけ出ないのか」を追えません。
        let found = parse("Host web\n  HostName web.example.test\n  Port にじゅうに\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].port, None);
    }

    #[test]
    fn it_reads_several() {
        let found = parse(
            "Host web\n  HostName web.example.test\n\nHost db\n  HostName db.example.test\n  User ops\n",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].id, "db");
        assert_eq!(found[1].user.as_deref(), Some("ops"));
    }

    #[test]
    fn missing_is_not_the_same_as_empty() {
        // **読めない一覧を空として扱いません。**0 件と「無い」は別です。
        let error = read(Path::new("/nope/does-not-exist")).expect_err("在るはずがない");
        assert!(matches!(error, SshConfigError::Missing));
    }
}
