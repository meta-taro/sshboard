//! 資格情報を **OS 資格情報ストアと ssh-agent へ委譲する**（D11）。
//!
//! **自前の鍵ストアを作りません。**作った瞬間に漏洩の責任を製品が引き受けます。
//! 持たなければ守らなくてよい。

mod agent;
mod agent_protocol;
mod secrets;

pub use agent::{list_identities, AgentConnectError};
pub use agent_protocol::{
    fingerprint, parse_identities, request_identities, AgentError, AgentIdentity,
};
pub use secrets::{SecretError, SecretStore};
