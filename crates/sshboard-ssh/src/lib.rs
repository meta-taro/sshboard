//! SSH 1 本の上で `sftp` と `exec` を扱う層。
//!
//! **SFTP の実装を 2 つ持ちません。裏で見えないセッションを張りません**（PRD §4-1）。
//! **ホスト鍵を必ず検証します**（D6）。

mod hostkey;
mod key_file;
mod session;
mod write_scope;

pub use hostkey::{decide, fingerprint, fingerprints_for, SeenHostKey, Trust};
pub use key_file::{inspect_key, KeyFacts, KeyFormat, KeyVerdict};
pub use session::{Auth, Console, DirEntry, Ran, SshError, SshSession, Target};
pub use write_scope::{Refusal, WriteScope};
