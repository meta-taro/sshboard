//! `russh`（純 Rust）で試す。
//!
//! **1 本のセッションの上で** `exec` と `sftp` の両方を通す（Issue 002 の完了条件）。
//! SFTP は russh 本体に無いので `russh-sftp` を重ねる。**そこが ssh2 との差。**

use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::ChannelMsg;
use russh_sftp::client::SftpSession;

use crate::report::BackendReport;
use crate::sniff;
use crate::{Auth, EXEC_MARKER};

const TIMEOUT: Duration = Duration::from_secs(15);

/// ホスト鍵を受け取って覚えるだけのハンドラ。
///
/// **受け入れる。**探り棒なので繋がるかを見るのが目的で、
/// 指紋を出力して**人が known_hosts と突き合わせる**前提にしている。
/// 製品側では受け入れない（russh の既定は全部拒否・dbboard ADR-0069 と同じ扱いにする）。
struct Probe {
    fingerprint: Arc<Mutex<Option<String>>>,
}

impl client::Handler for Probe {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let seen = match server_public_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => {
                key.fingerprint(Default::default()).to_string()
            }
            PublicKeyOrCertificate::Certificate(certificate) => {
                format!("certificate({})", certificate.key_id())
            }
        };
        if let Ok(mut held) = self.fingerprint.lock() {
            *held = Some(seen);
        }
        Ok(true)
    }
}

pub async fn run(
    host: &str,
    port: u16,
    user: &str,
    auth: &Auth,
    sftp_path: &str,
    sniff_path: Option<&str>,
) -> BackendReport {
    let mut report = BackendReport::new("russh (pure Rust) + russh-sftp");
    let fingerprint = Arc::new(Mutex::new(None));

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(TIMEOUT),
        ..Default::default()
    });

    let mut session = match client::connect(
        config,
        (host, port),
        Probe {
            fingerprint: Arc::clone(&fingerprint),
        },
    )
    .await
    {
        Ok(session) => session,
        Err(error) => {
            report.connected = Some(error.to_string());
            report.host_key_fingerprint = fingerprint.lock().ok().and_then(|held| held.clone());
            return report;
        }
    };

    report.host_key_fingerprint = fingerprint.lock().ok().and_then(|held| held.clone());

    if let Err(error) = authenticate(&mut session, user, auth).await {
        report.authenticated = Some(error);
        return report;
    }

    report.exec = exec(&session).await;
    report.sftp_entries = sftp_ls(&session, sftp_path).await;
    if let Some(path) = sniff_path {
        report.sniff = Some(sniff_remote(&session, path).await);
    }

    report
}

async fn authenticate(session: &mut Handle<Probe>, user: &str, auth: &Auth) -> Result<(), String> {
    let result = match auth {
        Auth::Agent => {
            let mut agent = russh::keys::agent::client::AgentClient::connect_env()
                .await
                .map_err(|error| format!("ssh-agent へ繋がりません: {error}"))?;
            let identities = agent
                .request_identities()
                .await
                .map_err(|error| format!("ssh-agent から鍵を読めません: {error}"))?;

            let key = identities
                .into_iter()
                .find_map(|identity| match identity {
                    russh::keys::agent::AgentIdentity::PublicKey { key, .. } => Some(key),
                    russh::keys::agent::AgentIdentity::Certificate { .. } => None,
                })
                .ok_or_else(|| "ssh-agent に鍵がありません".to_string())?;

            session
                .authenticate_publickey_with(user, key, None, &mut agent)
                .await
                .map_err(|error| error.to_string())?
        }
        Auth::Key { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|error| format!("鍵を読めません: {error}"))?;
            let hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(|error| error.to_string())?
                .flatten();
            session
                .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                .await
                .map_err(|error| error.to_string())?
        }
    };

    if result.success() {
        Ok(())
    } else {
        Err("認証が通りませんでした".into())
    }
}

async fn exec(session: &Handle<Probe>) -> Result<String, String> {
    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .exec(true, format!("echo {EXEC_MARKER}"))
        .await
        .map_err(|e| e.to_string())?;

    let mut output = Vec::new();
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { ref data } => output.extend_from_slice(data),
            ChannelMsg::ExitStatus { .. } => {}
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }

    let text = String::from_utf8_lossy(&output).trim().to_string();
    if text == EXEC_MARKER {
        Ok(text)
    } else {
        // 中身をそのまま出さない。想定外の出力に接続先が混ざりうる。
        Err(format!("想定と違う出力（{} バイト）", output.len()))
    }
}

/// 同じセッションの上に sftp サブシステムを開く。**2 本目の接続を張らない。**
async fn open_sftp(session: &Handle<Probe>) -> Result<SftpSession, String> {
    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;
    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| e.to_string())
}

async fn sftp_ls(session: &Handle<Probe>, path: &str) -> Result<usize, String> {
    let sftp = open_sftp(session).await?;
    let entries = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
    Ok(entries.count())
}

async fn sniff_remote(session: &Handle<Probe>, path: &str) -> Result<sniff::Sniff, String> {
    use tokio::io::AsyncReadExt;

    let sftp = open_sftp(session).await?;
    let mut file = sftp.open(path).await.map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    Ok(sniff::sniff(&bytes))
}
