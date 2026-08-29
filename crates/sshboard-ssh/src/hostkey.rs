//! ホスト鍵の検証。
//!
//! **blind-accept を型で作れない形にします**（D6 / dbboard ADR-0069）。
//! `russh` の `check_server_key` は既定で全部拒否するので、
//! **こちらが明示的に「なぜ通してよいか」を返さない限り繋がりません。**
//!
//! **指紋だけを固定しないこと。**同じサーバーでも、成立したホスト鍵の方式が違えば
//! 指紋も違います（実機で踏んだ・Issue 002）。方式と一緒に記録します。

use base64::Engine;
use sha2::{Digest, Sha256};

/// SSH の既定ポート。**この番号のときだけ、素のホスト名も見る。**
const DEFAULT_SSH_PORT: u16 = 22;

/// 見たホスト鍵。**方式と指紋の対**で扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenHostKey {
    /// `ssh-ed25519` / `ecdsa-sha2-nistp256` など。
    pub algorithm: String,
    /// `SHA256:...`（`ssh-keygen -l` と同じ形）。
    pub fingerprint: String,
}

/// 通してよい理由。**「とりあえず通す」を作らない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trust {
    /// 登録済みの指紋と一致した。
    Pinned,
    /// `known_hosts` に載っていた。
    KnownHosts,
    /// **初めて見るホスト。**人が確かめる必要がある。
    Unknown,
    /// **登録と食い違う。**すり替えの可能性。
    Mismatch { expected: String },
}

impl Trust {
    /// 繋いでよいか。**`Unknown` と `Mismatch` は通さない。**
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Trust::Pinned | Trust::KnownHosts)
    }
}

/// 公開鍵の生バイトから `ssh-keygen -l` と同じ形の指紋を作る。
pub fn fingerprint(key_blob: &[u8]) -> String {
    let digest = Sha256::digest(key_blob);
    format!(
        "SHA256:{}",
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest)
    )
}

/// 見た鍵を、登録済みの指紋と `known_hosts` に照らす。
///
/// - `pinned` … 接続に記録された指紋（`ConnectionEntry::fingerprint`）
/// - `known` … `known_hosts` から拾った、そのホストの指紋
pub fn decide(seen: &SeenHostKey, pinned: Option<&str>, known: &[String]) -> Trust {
    if let Some(expected) = pinned {
        return if expected == seen.fingerprint {
            Trust::Pinned
        } else {
            Trust::Mismatch {
                expected: expected.to_owned(),
            }
        };
    }

    if known.iter().any(|entry| entry == &seen.fingerprint) {
        return Trust::KnownHosts;
    }

    Trust::Unknown
}

/// `|1|<salt>|<hash>` 形式の 1 件が、この名前と一致するか。
///
/// **OpenSSH は既定でホスト名をハッシュ化して保存します**（`HashKnownHosts yes`）。
/// ここを読めないと、**既に ssh で入ったことのあるホストが全部「初めて見る」になり**、
/// 人が確かめて済ませた判断を捨てることになります。
///
/// 中身は `HMAC-SHA1(key = salt, message = ホスト名)`。
/// 鍵の強度が要る用途ではありませんが、**照合はこの形でしかできません。**
fn hashed_matches(entry: &str, name: &str) -> bool {
    let Some(rest) = entry.strip_prefix("|1|") else {
        return false;
    };
    let Some((salt, expected)) = rest.split_once('|') else {
        return false;
    };

    let engine = base64::engine::general_purpose::STANDARD;
    let (Ok(salt), Ok(expected)) = (engine.decode(salt), engine.decode(expected)) else {
        return false;
    };

    hmac_sha1(&salt, name.as_bytes()) == expected.as_slice()
}

/// HMAC-SHA1。**`known_hosts` の照合にしか使いません。**
fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    use sha1::{Digest, Sha1};

    const BLOCK: usize = 64;

    // 鍵がブロックより長ければ縮める。短ければ 0 で伸ばす（RFC 2104）。
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..20].copy_from_slice(&Sha1::digest(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for index in 0..BLOCK {
        inner_pad[index] ^= padded[index];
        outer_pad[index] ^= padded[index];
    }

    let mut inner = Sha1::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner = inner.finalize();

    let mut outer = Sha1::new();
    outer.update(outer_pad);
    outer.update(inner);
    outer.finalize().into()
}

/// `known_hosts` から、そのホストに対応する指紋を拾う。
///
/// **ハッシュ化された行（`|1|...`）も読みます。**OpenSSH の既定がハッシュ化なので、
/// ここを諦めると「既に ssh で入ったことのあるホストが全部初めて見るホストになる」。
pub fn fingerprints_for(known_hosts: &str, host: &str, port: u16) -> Vec<String> {
    // **既定ポート以外では、素のホスト名を見ない。**
    // OpenSSH が `[host]:port` の形でしか記録しないため、素の名前も見てしまうと
    // **ポート 22 で確かめた鍵を、別ポートの相手に流用する**ことになる。
    let looking_for: Vec<String> = if port == DEFAULT_SSH_PORT {
        vec![host.to_owned(), format!("[{host}]:{port}")]
    } else {
        vec![format!("[{host}]:{port}")]
    };

    known_hosts
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hosts = parts.next()?;
            let _algorithm = parts.next()?;
            let key = parts.next()?;

            let matches = hosts.split(',').any(|entry| {
                looking_for
                    .iter()
                    .any(|name| entry == name || hashed_matches(entry, name))
            });
            if !matches {
                return None;
            }

            let blob = base64::engine::general_purpose::STANDARD.decode(key).ok()?;
            Some(fingerprint(&blob))
        })
        .collect()
}
