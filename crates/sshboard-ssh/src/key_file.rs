//! 鍵ファイルが何なのかを、**中身だけで**見分ける（D28）。
//!
//! **拡張子を信用しません。**実物で裏切られたからです — `*.tera.ppk` という
//! 名前のファイルの中身が OpenSSH 秘密鍵で、拡張子で判定していた製品は
//! 「PuTTY 形式です。変換してください」と、**要らない作業へ人を送っていました**。
//!
//! **変換もしません。**`russh` が PPK v2 / v3 をそのまま読めるので、
//! 変換すべきものが最初から無い。変換物をディスクへ置かない方が安全でもあります
//! （product-baseline §14 ／ D11「自前の鍵ストアを作らない」）。
//!
//! **鍵の中身はここを通り抜けるだけです。**保持も記録もしません。

/// 鍵ファイルの形式。**人に見せる名前でもあります。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFormat {
    /// OpenSSH 形式（`-----BEGIN OPENSSH PRIVATE KEY-----`）。
    OpenSsh,
    /// PuTTY 形式 v2。Tera Term / WinSCP の層が持っている（D19）。
    Ppk2,
    /// PuTTY 形式 v3。鍵の導出に Argon2 を使う。
    Ppk3,
    /// 古い PEM（`BEGIN RSA PRIVATE KEY`）。実機に残っている（Issue 002）。
    Pkcs1,
    /// PKCS#8（`BEGIN PRIVATE KEY` / `BEGIN ENCRYPTED PRIVATE KEY`）。
    Pkcs8,
    /// **公開鍵。**取り違えが一番多いので、名前を付けて断る。
    PublicKey,
    /// 鍵に見えない。
    Unknown,
}

impl KeyFormat {
    /// 人に見せる短い名前。**そのまま画面へ出せます。**
    pub fn label(self) -> &'static str {
        match self {
            KeyFormat::OpenSsh => "OpenSSH",
            KeyFormat::Ppk2 => "PuTTY (PPK v2)",
            KeyFormat::Ppk3 => "PuTTY (PPK v3)",
            KeyFormat::Pkcs1 => "PEM (PKCS#1)",
            KeyFormat::Pkcs8 => "PKCS#8",
            KeyFormat::PublicKey => "public key",
            KeyFormat::Unknown => "unknown",
        }
    }
}

/// 鍵ファイルについて分かったこと。
///
/// **「読めるか」と「パスフレーズが要るか」は別の話**です。
/// 読めない鍵にパスフレーズを聞くと、人は入れ続けて理由に辿り着けません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyFacts {
    pub format: KeyFormat,
    /// **この製品が認証に使えるか。**公開鍵と、鍵でないものは使えない。
    pub usable: bool,
    /// パスフレーズが要るか。**要らないものに聞かない**（聞くこと自体が壁になる）。
    pub needs_passphrase: bool,
}

/// 見出しだけを読んで判定する。**鍵の中身は解釈しません。**
///
/// 判定を誤っても、最悪「パスフレーズを聞かずに正直に失敗する」だけになるよう、
/// **素朴な見出し合わせ**にしてあります。鍵の形式を自前で解釈し始めると
/// そこが事故の場所になる（D19 が心配していた点）。
pub fn inspect_key(bytes: &[u8]) -> KeyFacts {
    // 鍵は必ずテキストです。読めないバイトがあっても**落とさず**、置換して進みます。
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}').trim_start();

    if let Some(facts) = putty(text) {
        return facts;
    }
    if text.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----") {
        return usable(KeyFormat::OpenSsh, !openssh_body_is_unencrypted(text));
    }
    if text.starts_with("-----BEGIN RSA PRIVATE KEY-----")
        || text.starts_with("-----BEGIN DSA PRIVATE KEY-----")
        || text.starts_with("-----BEGIN EC PRIVATE KEY-----")
    {
        return usable(KeyFormat::Pkcs1, text.contains("Proc-Type: 4,ENCRYPTED"));
    }
    if text.starts_with("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return usable(KeyFormat::Pkcs8, true);
    }
    if text.starts_with("-----BEGIN PRIVATE KEY-----") {
        return usable(KeyFormat::Pkcs8, false);
    }
    if is_public_key(text) {
        return refused(KeyFormat::PublicKey);
    }
    refused(KeyFormat::Unknown)
}

/// PuTTY 形式かどうか。版と、暗号化の有無を見出しから読む。
fn putty(text: &str) -> Option<KeyFacts> {
    let rest = text.strip_prefix("PuTTY-User-Key-File-")?;
    let format = match rest.as_bytes().first() {
        Some(b'3') => KeyFormat::Ppk3,
        // 版が読めなくても PuTTY 形式だとは分かる。**分かる所までは伝える。**
        _ => KeyFormat::Ppk2,
    };

    // `Encryption: none` 以外は、すべてパスフレーズが要ります。
    // **見つからなければ「要る」側へ倒す。**聞いて余るより、聞かずに詰まる方が悪い。
    let encrypted = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("Encryption:"))
        .map(|value| value.trim() != "none")
        .unwrap_or(true);

    Some(usable(format, encrypted))
}

/// 公開鍵か。**`.pub` を指す取り違えが一番多い。**
fn is_public_key(text: &str) -> bool {
    let head = text.split_whitespace().next().unwrap_or_default();
    head.starts_with("ssh-")
        || head.starts_with("ecdsa-sha2-")
        || head.starts_with("sk-ssh-")
        || head.starts_with("sk-ecdsa-")
}

/// OpenSSH 形式の本文が素のままか。
///
/// 本文の先頭は base64 で `openssh-key-v1\0` ＋ 暗号方式名です。
/// **暗号方式が `none` の鍵だけ**が、必ずこの前置きになります。
fn openssh_body_is_unencrypted(text: &str) -> bool {
    const NONE_PREFIX: &str = "b3BlbnNzaC1rZXktdjEAAAAABG5vbmU";
    text.lines().any(|line| line.starts_with(NONE_PREFIX))
}

fn usable(format: KeyFormat, needs_passphrase: bool) -> KeyFacts {
    KeyFacts {
        format,
        usable: true,
        needs_passphrase,
    }
}

fn refused(format: KeyFormat) -> KeyFacts {
    KeyFacts {
        format,
        usable: false,
        needs_passphrase: false,
    }
}
