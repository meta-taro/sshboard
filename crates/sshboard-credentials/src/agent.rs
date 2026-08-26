//! 動いている ssh-agent に「どの鍵を持っているか」を尋ねる。
//!
//! **秘密鍵は受け取りません。**受け取るのは公開鍵の指紋とコメントだけです（D11）。
//! 署名は agent の中で行われ、**パスフレーズを製品が一度も受け取りません。**

use std::io::{Read, Write};

use crate::agent_protocol::{self, AgentError, AgentIdentity};

/// agent の返事の上限。鍵が数十本でも数 KB で収まる。
const MAX_REPLY: usize = 256 * 1024;

/// 尋ねられなかった理由。**握り潰さない。**
#[derive(Debug)]
pub enum AgentConnectError {
    /// agent が動いていない（`SSH_AUTH_SOCK` が無い等）。**異常ではない。**
    NotRunning {
        detail: String,
    },
    Io(String),
    Protocol(AgentError),
}

impl std::fmt::Display for AgentConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentConnectError::NotRunning { detail } => {
                write!(f, "ssh-agent が見つかりません: {detail}")
            }
            AgentConnectError::Io(detail) => write!(f, "ssh-agent と話せません: {detail}"),
            AgentConnectError::Protocol(error) => {
                write!(f, "ssh-agent の返事を読めません: {error}")
            }
        }
    }
}

impl std::error::Error for AgentConnectError {}

/// agent が持っている鍵の一覧。
pub fn list_identities() -> Result<Vec<AgentIdentity>, AgentConnectError> {
    let payload = ask()?;
    agent_protocol::parse_identities(&payload).map_err(AgentConnectError::Protocol)
}

/// 要求を書いて、返事の payload を読む。
fn exchange(io: &mut (impl Read + Write)) -> Result<Vec<u8>, AgentConnectError> {
    io.write_all(&agent_protocol::request_identities())
        .and_then(|()| io.flush())
        .map_err(|error| AgentConnectError::Io(error.to_string()))?;

    let mut length = [0u8; 4];
    io.read_exact(&mut length)
        .map_err(|error| AgentConnectError::Io(error.to_string()))?;
    let len = u32::from_be_bytes(length) as usize;

    if len == 0 || len > MAX_REPLY {
        return Err(AgentConnectError::Io(format!(
            "返事の長さが異常です: {len}"
        )));
    }

    let mut payload = vec![0u8; len];
    io.read_exact(&mut payload)
        .map_err(|error| AgentConnectError::Io(error.to_string()))?;
    Ok(payload)
}

#[cfg(unix)]
fn ask() -> Result<Vec<u8>, AgentConnectError> {
    use std::os::unix::net::UnixStream;

    let path = std::env::var("SSH_AUTH_SOCK").map_err(|_| AgentConnectError::NotRunning {
        detail: "SSH_AUTH_SOCK が設定されていません".to_owned(),
    })?;

    let mut socket = UnixStream::connect(&path).map_err(|error| AgentConnectError::NotRunning {
        detail: error.to_string(),
    })?;

    exchange(&mut socket)
}

#[cfg(windows)]
fn ask() -> Result<Vec<u8>, AgentConnectError> {
    use std::fs::OpenOptions;

    // Windows の OpenSSH agent は名前付きパイプ。読み書き両方で開けば、
    // 追加の依存を足さずにファイルとして扱える。
    const PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

    let mut pipe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE)
        .map_err(|error| AgentConnectError::NotRunning {
            detail: error.to_string(),
        })?;

    exchange(&mut pipe)
}

#[cfg(not(any(unix, windows)))]
fn ask() -> Result<Vec<u8>, AgentConnectError> {
    Err(AgentConnectError::NotRunning {
        detail: "この OS には対応していません".to_owned(),
    })
}
