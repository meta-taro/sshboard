//! SSH 1 本の上で `sftp` と `exec` を扱う層。
//!
//! **SFTP の実装を 2 つ持ちません。裏で見えないセッションを張りません**（PRD §4-1）。
//! **ホスト鍵を必ず検証します**（D6）。

mod hostkey;
mod session;

pub use hostkey::{decide, fingerprint, fingerprints_for, SeenHostKey, Trust};
pub use session::{Auth, SshError, SshSession, Target};
