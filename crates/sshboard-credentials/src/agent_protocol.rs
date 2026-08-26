//! ssh-agent プロトコルのうち、**鍵の一覧を尋ねる部分だけ**。
//!
//! **なぜ自前で書くか**: D6（SSH ライブラリ）がまだ決まっていない。
//! ここを russh や ssh2 に依存させると、**D6 を実装の都合で先に決めてしまう。**
//! 尋ねるのは 1 種類のメッセージだけなので、それより自分で書く方が安い。
//!
//! **秘密鍵はここを通りません。**agent が返すのは公開鍵とコメントだけです（D11）。

use base64::Engine;
use sha2::{Digest, Sha256};

/// 「鍵の一覧をください」（draft-miller-ssh-agent）。
pub const SSH_AGENTC_REQUEST_IDENTITIES: u8 = 11;
/// その答え。
pub const SSH_AGENT_IDENTITIES_ANSWER: u8 = 12;

/// agent が持っている鍵 1 本の**識別**。
///
/// **秘密鍵も、パスフレーズも、公開鍵の生バイトも持ちません。**
/// 人が「どの鍵か」を選べるだけの情報に絞ります（D11）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    /// `SHA256:...`。`ssh-add -l` が出すものと同じ形。
    pub fingerprint: String,
    /// 鍵に付いているコメント。**利用者が付けた文字列なので、そのまま画面へ出さない。**
    pub comment: String,
}

/// 読めなかった理由。**握り潰さない。**
#[derive(Debug, PartialEq, Eq)]
pub enum AgentError {
    /// 期待した種類の返事ではない。
    UnexpectedMessage { kind: u8 },
    /// 途中で尽きた。
    Truncated { at: usize },
    /// コメントが UTF-8 でない。
    CommentNotUtf8 { index: usize },
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::UnexpectedMessage { kind } => {
                write!(f, "想定外の返事です（種類 {kind}）")
            }
            AgentError::Truncated { at } => write!(f, "{at} バイト目で尽きました"),
            AgentError::CommentNotUtf8 { index } => {
                write!(f, "{index} 番目の鍵のコメントが UTF-8 ではありません")
            }
        }
    }
}

impl std::error::Error for AgentError {}

/// 「鍵の一覧をください」を組み立てる。長さ 4 バイト ＋ 種類 1 バイト。
pub fn request_identities() -> Vec<u8> {
    let mut out = 1u32.to_be_bytes().to_vec();
    out.push(SSH_AGENTC_REQUEST_IDENTITIES);
    out
}

/// 返事の payload（長さの 4 バイトを除いた部分）を読む。
pub fn parse_identities(payload: &[u8]) -> Result<Vec<AgentIdentity>, AgentError> {
    let kind = *payload.first().ok_or(AgentError::Truncated { at: 0 })?;
    if kind != SSH_AGENT_IDENTITIES_ANSWER {
        return Err(AgentError::UnexpectedMessage { kind });
    }

    let raw_count = payload.get(1..5).ok_or(AgentError::Truncated { at: 1 })?;
    let count = u32::from_be_bytes(raw_count.try_into().expect("4 バイト取れている")) as usize;

    // **`count` で確保しない。**件数だけ大きい返事で確保を膨らませないため、
    // 実際に読めた分だけ積む。
    let mut identities = Vec::new();
    let mut at = 5;

    for index in 0..count {
        let (blob, after_blob) = read_string(payload, at)?;
        let (raw_comment, after_comment) = read_string(payload, after_blob)?;

        let comment =
            std::str::from_utf8(raw_comment).map_err(|_| AgentError::CommentNotUtf8 { index })?;

        // **公開鍵の生バイトは持ち歩かない。**指紋にしてから捨てる（D11）。
        identities.push(AgentIdentity {
            fingerprint: fingerprint(blob),
            comment: comment.to_owned(),
        });
        at = after_comment;
    }

    Ok(identities)
}

/// 公開鍵の生バイトから `ssh-add -l` と同じ形の指紋を作る。
pub fn fingerprint(key_blob: &[u8]) -> String {
    let digest = Sha256::digest(key_blob);
    format!(
        "SHA256:{}",
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest)
    )
}

/// `uint32(長さ) + 本体` を 1 つ読み、次の位置を返す。
fn read_string(bytes: &[u8], at: usize) -> Result<(&[u8], usize), AgentError> {
    let after_len = at + 4;
    let raw_len = bytes
        .get(at..after_len)
        .ok_or(AgentError::Truncated { at })?;
    let len = u32::from_be_bytes(raw_len.try_into().expect("4 バイト取れている")) as usize;

    let end = after_len
        .checked_add(len)
        .ok_or(AgentError::Truncated { at: after_len })?;
    let body = bytes
        .get(after_len..end)
        .ok_or(AgentError::Truncated { at: after_len })?;

    Ok((body, end))
}
