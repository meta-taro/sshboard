//! SSH 1 本。**この上に `sftp` と `exec` を載せます。**
//!
//! **2 本目を張りません**（PRD §4-1「裏で見えないセッションを張らない」）。
//! **すべての操作が帯に出ます**（PRD §4-2）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{keys, ChannelMsg};
use sshboard_band::{Actor, Band};

use crate::hostkey::{decide, fingerprint, fingerprints_for, SeenHostKey, Trust};

const TIMEOUT: Duration = Duration::from_secs(15);

/// 繋げなかった理由。**握り潰さない。**
#[derive(Debug)]
pub enum SshError {
    /// **ホスト鍵が信用できない。**初見か、登録と食い違う。
    UntrustedHost {
        seen: SeenHostKey,
        trust: Trust,
    },
    Connect(String),
    Authenticate(String),
    Command(String),
    /// 帯が受け取りを返さなかった。**見えないまま実行しない**（D16）。
    NotShown(String),
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::UntrustedHost { seen, trust } => match trust {
                Trust::Mismatch { expected } => write!(
                    f,
                    "ホスト鍵が登録と違います（{} / 見えたもの {}・登録 {expected}）",
                    seen.algorithm, seen.fingerprint
                ),
                _ => write!(
                    f,
                    "初めて見るホストです（{} / {}）。確かめて登録してください",
                    seen.algorithm, seen.fingerprint
                ),
            },
            SshError::Connect(detail) => write!(f, "繋がりません: {detail}"),
            SshError::Authenticate(detail) => write!(f, "認証が通りません: {detail}"),
            SshError::Command(detail) => write!(f, "コマンドが通りません: {detail}"),
            SshError::NotShown(detail) => write!(f, "画面へ出せませんでした: {detail}"),
        }
    }
}

impl std::error::Error for SshError {}

/// 認証のやり方。**パスワード認証を用意しません**（履歴と記憶に残る経路を作らない）。
pub enum Auth {
    /// ssh-agent へ委譲する。**パスフレーズを製品が受け取らない**（D11・推奨）。
    Agent,
    /// 鍵ファイル。パスフレーズは**人がその場で入れたものを渡す**。
    Key {
        path: String,
        passphrase: Option<String>,
    },
}

/// 繋ぐときに要るもの。**接続先はここにしか無い。**
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// 登録された指紋。**あればこちらを優先する。**
    pub pinned_fingerprint: Option<String>,
    /// `known_hosts` の中身。読めなければ空でよい。
    pub known_hosts: String,
}

/// ホスト鍵を見て覚えるだけのハンドラ。**判断は接続側が行う。**
struct Watcher {
    seen: Arc<Mutex<Option<SeenHostKey>>>,
}

impl client::Handler for Watcher {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        if let PublicKeyOrCertificate::PublicKey { key, .. } = server_public_key {
            let seen = SeenHostKey {
                algorithm: key.algorithm().to_string(),
                fingerprint: fingerprint(key.to_bytes().unwrap_or_default().as_slice()),
            };
            if let Ok(mut held) = self.seen.lock() {
                *held = Some(seen);
            }
        }
        // **ここでは通す。**判断は接続側で、記録した指紋と known_hosts に照らして行う。
        // 通したあとに信用できなければ、**セッションを捨てる。**
        Ok(true)
    }
}

/// 繋がっている 1 本。
pub struct SshSession {
    handle: Handle<Watcher>,
    band: Band,
    host_key: SeenHostKey,
}

impl SshSession {
    /// 繋いで、ホスト鍵を確かめる。
    ///
    /// **信用できないホストとは、繋いだあとでもセッションを捨てます。**
    pub async fn connect(target: &Target, auth: &Auth, band: Band) -> Result<Self, SshError> {
        let seen = Arc::new(Mutex::new(None));
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(TIMEOUT),
            ..Default::default()
        });

        let mut handle = client::connect(
            config,
            (target.host.as_str(), target.port),
            Watcher {
                seen: Arc::clone(&seen),
            },
        )
        .await
        .map_err(|error| SshError::Connect(error.to_string()))?;

        let host_key = seen
            .lock()
            .ok()
            .and_then(|held| held.clone())
            .ok_or_else(|| SshError::Connect("ホスト鍵を受け取れませんでした".into()))?;

        let known = fingerprints_for(&target.known_hosts, &target.host, target.port);
        let trust = decide(&host_key, target.pinned_fingerprint.as_deref(), &known);

        if !trust.is_acceptable() {
            // **繋いだあとでも捨てる。**信用できないまま使わない。
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "")
                .await;
            return Err(SshError::UntrustedHost {
                seen: host_key,
                trust,
            });
        }

        authenticate(&mut handle, &target.user, auth).await?;

        Ok(Self {
            handle,
            band,
            host_key,
        })
    }

    /// 繋がっている相手のホスト鍵。**人が known_hosts と突き合わせるため。**
    pub fn host_key(&self) -> &SeenHostKey {
        &self.host_key
    }

    /// コマンドを実行し、出力を返す。**先に帯へ出す**（D16）。
    pub async fn exec(&self, actor: Actor, command: &str) -> Result<String, SshError> {
        self.show(actor, &format!("$ {command}")).await?;

        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .exec(true, command)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        let mut out = Vec::new();
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { ref data } => out.extend_from_slice(data),
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// 帯へ載せ、画面が受け取るまで待つ（D16）。
    async fn show(&self, actor: Actor, text: &str) -> Result<(), SshError> {
        let delivery = self.band.record(actor, text);
        match delivery.wait_acked(Duration::from_secs(2)).await {
            sshboard_band::DeliveryOutcome::Delivered => Ok(()),
            sshboard_band::DeliveryOutcome::TimedOut { acked, expected } => {
                Err(SshError::NotShown(format!("{acked}/{expected}")))
            }
        }
    }
}

async fn authenticate(
    handle: &mut Handle<Watcher>,
    user: &str,
    auth: &Auth,
) -> Result<(), SshError> {
    let result = match auth {
        Auth::Agent => {
            let mut agent = keys::agent::client::AgentClient::connect_env()
                .await
                .map_err(|error| SshError::Authenticate(format!("ssh-agent: {error}")))?;
            let identities = agent
                .request_identities()
                .await
                .map_err(|error| SshError::Authenticate(format!("ssh-agent: {error}")))?;

            let mut last = None;
            for identity in identities {
                let keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
                    continue;
                };
                let attempt = handle
                    .authenticate_publickey_with(user, key, None, &mut agent)
                    .await
                    .map_err(|error| SshError::Authenticate(error.to_string()))?;
                if attempt.success() {
                    return Ok(());
                }
                last = Some(attempt);
            }
            last.ok_or_else(|| SshError::Authenticate("ssh-agent に鍵がありません".into()))?
        }
        Auth::Key { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|error| SshError::Authenticate(format!("鍵を読めません: {error}")))?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|error| SshError::Authenticate(error.to_string()))?
                .flatten();
            handle
                .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                .await
                .map_err(|error| SshError::Authenticate(error.to_string()))?
        }
    };

    if result.success() {
        Ok(())
    } else {
        Err(SshError::Authenticate(
            "鍵が受け付けられませんでした".into(),
        ))
    }
}

/// `sftp` で見えたもの 1 件。**中身は持ちません。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

impl SshSession {
    /// 同じセッションの上に `sftp` を開く。**2 本目の接続を張りません。**
    async fn sftp(&self) -> Result<russh_sftp::client::SftpSession, SshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| SshError::Command(error.to_string()))
    }

    /// ディレクトリの一覧。**先に帯へ出す**（D16）。
    pub async fn list_dir(&self, actor: Actor, path: &str) -> Result<Vec<DirEntry>, SshError> {
        self.show(actor, &format!("ls {path}")).await?;

        let sftp = self.sftp().await?;
        let entries = sftp
            .read_dir(path)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        Ok(entries
            .map(|entry| DirEntry {
                name: entry.file_name(),
                is_dir: entry.file_type().is_dir(),
                size: entry.metadata().size.unwrap_or(0),
            })
            .collect())
    }

    /// ファイルを丸ごと読む。**バイト列のまま返します。**
    ///
    /// **ここで文字コードを決めません。**EUC-JP のログが実在するので
    /// （Issue 002・手元のテスト用サーバーでも再現済み）、
    /// **変換するかどうかは呼び出し側が決めます。**
    pub async fn read_file(&self, actor: Actor, path: &str) -> Result<Vec<u8>, SshError> {
        use tokio::io::AsyncReadExt;

        self.show(actor, &format!("read {path}")).await?;

        let sftp = self.sftp().await?;
        let mut file = sftp
            .open(path)
            .await
            .map_err(|e| SshError::Command(e.to_string()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        Ok(bytes)
    }

    /// ログを追う。**同じ 1 本の出力を、GUI へは生・MCP へは素で流します**（Issue 005）。
    ///
    /// 人が止めたら（`OutputStream::stop`）、**その場で追うのをやめます**（PRD §4-3）。
    pub async fn follow(
        &self,
        actor: Actor,
        path: &str,
        lines: u32,
        into: Arc<sshboard_stream::OutputStream>,
    ) -> Result<(), SshError> {
        // 引数を組み立てるのは**こちら**で、外から任意の文字列を渡させない（D3）。
        let command = format!("tail -n {lines} -f {}", shell_quote(path));
        self.show(actor, &format!("$ {command}")).await?;

        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .exec(true, command)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { ref data } => {
                    // 人が止めたら、そこで終わり（PRD §4-3）。
                    if into.push(data).is_err() {
                        break;
                    }
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        let _ = channel.close().await;
        Ok(())
    }
}

/// パスを 1 語として渡す。**`run_command` を作らないための下ごしらえ**（D3）。
///
/// 単引用符で囲み、中の単引用符だけを閉じ直す。
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
