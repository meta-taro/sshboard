//! 暗号化された PKCS#8 を、**`russh` が本当に読めるか**だけ見分ける（D28）。
//!
//! `key_file.rs` は見出しだけを読む方針ですが、ここだけは例外です。
//! **見出し（`BEGIN ENCRYPTED PRIVATE KEY`）が同じまま、読めるものと読めないものが
//! 混ざる**ため、見出しでは分けられません。一律で断っていた結果、
//! **Linux / Windows の ssh-keygen が作る鍵をまるごと拒んでいました。**
//! これは D28 が自分で挙げた「止めるべき条件（読める → 断ってはいけない）」です。
//!
//! **鍵の中身（暗号文）は一切見ません。**見るのは先頭の
//! `encryptionAlgorithm`（暗号方式の名前）だけで、ここは秘密ではありません。
//!
//! ## 実測（2026-09-02・`russh` 0.63・16 検体）
//!
//! **読めたのは、次の組み合わせだけでした。**
//!
//! ```text
//! PBES2 ＋ aes-128/192/256-cbc ＋ （PBKDF2 の PRF が hmacWithSHA256 以上／または scrypt）
//! ```
//!
//! 読めなかったもの:
//!
//! | 方式 | 断られ方 |
//! |---|---|
//! | PBKDF2 の PRF が hmacWithSHA1（**省略時の既定**） | `PKCS#5 algorithm 1.2.840.113549.2.7 is unsupported` |
//! | des-ede3-cbc（PRF によらず） | `unknown/unsupported OID: 1.2.840.113549.3.7` |
//! | PBES1 / PKCS#12 PBE（MD5-DES・SHA1-3DES・SHA1-RC2-40） | `Could not read key` ほか |
//!
//! **どれもパニックしません。**`russh` は必ず `Err` を返します
//! （PKCS#1 の AES-256-CBC とはここが違う。あちらは `unimplemented!()` に落ちます）。
//!
//! ## なぜ許可リストなのか
//!
//! `russh` の PKCS#5 実装は**元から狭い**ので、「読める側」を数え上げる方が実物に近い。
//! **`russh` が対応を広げたら、`tests/key_formats_really_load.rs` が落ちて教えます。**
//! 判定を緩めるのはそのときで、いま先回りしない。
//!
//! ただし **DER を解けなかったときだけは「使える」に倒します。**
//! 方式については実測が揃っていますが、解析できなかった鍵については
//! **何も分かっていない**からです。分からないまま断ると、正しい鍵が行き止まりになります。

use base64::Engine as _;

/// OID の DER 表現（タグと長さを含む）。**中身を解釈せず、この並びを探すだけ。**
const OID_PBES2: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x05, 0x0D,
];
const OID_PBKDF2: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x05, 0x0C,
];
/// `1.3.6.1.4.1.11591.4.11`
const OID_SCRYPT: &[u8] = &[
    0x06, 0x09, 0x2B, 0x06, 0x01, 0x04, 0x01, 0xDA, 0x47, 0x04, 0x0B,
];

/// `2.16.840.1.101.3.4.1.{2,22,42}`
const OID_AES128_CBC: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x02,
];
const OID_AES192_CBC: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x16,
];
const OID_AES256_CBC: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x2A,
];

/// `russh` が読める PRF。**hmacWithSHA1 はここに無い**（実測で読めない）。
const OID_HMAC_SHA256: &[u8] = &[0x06, 0x08, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x09];
const OID_HMAC_SHA384: &[u8] = &[0x06, 0x08, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x0A];
const OID_HMAC_SHA512: &[u8] = &[0x06, 0x08, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x0B];

/// この PEM を `russh` が読めるか。
pub(crate) fn encrypted_pkcs8_is_readable(pem: &str) -> bool {
    let Some(der) = decode_pem_body(pem) else {
        return true; // 解けなかった。**何も分かっていないので通す**（上の注記）。
    };
    let Some(algorithm) = encryption_algorithm(&der) else {
        return true;
    };
    algorithm_is_readable(algorithm)
}

/// `encryptionAlgorithm` の並びから、読めるかどうかを決める。**許可リスト。**
fn algorithm_is_readable(algorithm: &[u8]) -> bool {
    let has = |oid: &[u8]| contains(algorithm, oid);

    // PBES2 以外（PBES1 / PKCS#12 PBE）は、実測でどれも読めなかった。
    if !has(OID_PBES2) {
        return false;
    }
    // 暗号は AES-CBC だけ。**3DES は PRF によらず読めない。**
    if !has(OID_AES128_CBC) && !has(OID_AES192_CBC) && !has(OID_AES256_CBC) {
        return false;
    }
    // 鍵導出。PBKDF2 なら、PRF が明示されていて SHA256 以上であること。
    // **省略すると hmacWithSHA1 が既定になり、読めない。**
    if has(OID_PBKDF2) {
        return has(OID_HMAC_SHA256) || has(OID_HMAC_SHA384) || has(OID_HMAC_SHA512);
    }
    has(OID_SCRYPT)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// PEM の本体を DER へ戻す。**見出しの間だけを取ります。**
fn decode_pem_body(pem: &str) -> Option<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN ENCRYPTED PRIVATE KEY-----";
    const END: &str = "-----END ENCRYPTED PRIVATE KEY-----";
    let start = pem.find(BEGIN)? + BEGIN.len();
    let end = pem[start..].find(END)? + start;
    let body: String = pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64::engine::general_purpose::STANDARD.decode(body).ok()
}

/// `EncryptedPrivateKeyInfo ::= SEQUENCE { encryptionAlgorithm, encryptedData }`
/// の **1 つめ**を切り出す。**暗号文（2 つめ）には触れません。**
fn encryption_algorithm(der: &[u8]) -> Option<&[u8]> {
    let outer = read_tlv(der)?;
    if outer.tag != 0x30 {
        return None;
    }
    Some(read_tlv(outer.value)?.value)
}

struct Tlv<'a> {
    tag: u8,
    value: &'a [u8],
}

/// DER の 1 要素を読む。**長さの形（短形式 / 長形式）だけを解釈します。**
fn read_tlv(bytes: &[u8]) -> Option<Tlv<'_>> {
    let tag = *bytes.first()?;
    let first_len = *bytes.get(1)? as usize;
    let (len, header) = if first_len < 0x80 {
        (first_len, 2)
    } else {
        let count = first_len & 0x7F;
        // 長さの長さが 4 バイトを超える鍵は無い。**そこまで来たら諦めます。**
        if count == 0 || count > 4 {
            return None;
        }
        let mut len = 0usize;
        for i in 0..count {
            len = (len << 8) | *bytes.get(2 + i)? as usize;
        }
        (len, 2 + count)
    };
    let value = bytes.get(header..header.checked_add(len)?)?;
    Some(Tlv { tag, value })
}
