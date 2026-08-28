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

/// `known_hosts` から、そのホストに対応する指紋を拾う。
///
/// **ハッシュ化された行（`|1|...`）は読めません。**読めないものを
/// 「載っていない」と扱い、**通す方向へ倒しません。**
pub fn fingerprints_for(known_hosts: &str, host: &str, port: u16) -> Vec<String> {
    let plain = host.to_owned();
    let with_port = format!("[{host}]:{port}");

    known_hosts
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hosts = parts.next()?;
            let _algorithm = parts.next()?;
            let key = parts.next()?;

            let matches = hosts.split(',').any(|entry| {
                entry == plain || entry == with_port || (port == 22 && entry == plain)
            });
            if !matches {
                return None;
            }

            let blob = base64::engine::general_purpose::STANDARD.decode(key).ok()?;
            Some(fingerprint(&blob))
        })
        .collect()
}
