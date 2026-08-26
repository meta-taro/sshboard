//! `ssh2`（libssh2 バインディング）で試す。
//!
//! **1 本のセッションの上で** `exec` と `sftp` の両方を通す（Issue 002 の完了条件）。
//! libssh2 は成立した方式を教えてくれるので、そこが russh 側より厚い。

use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use base64::Engine;
use ssh2::{HashType, MethodType, Session};

use crate::report::BackendReport;
use crate::sniff;
use crate::{Auth, EXEC_MARKER};

const TIMEOUT: Duration = Duration::from_secs(15);

pub fn run(
    host: &str,
    port: u16,
    user: &str,
    auth: &Auth,
    sftp_path: &str,
    sniff_path: Option<&str>,
) -> BackendReport {
    let mut report = BackendReport::new("ssh2 (libssh2)");

    let session = match connect(host, port) {
        Ok(session) => session,
        Err(error) => {
            report.connected = Some(error);
            return report;
        }
    };

    report.host_key_fingerprint = fingerprint(&session);
    report.negotiated = negotiated(&session);

    if let Err(error) = authenticate(&session, user, auth) {
        report.authenticated = Some(error);
        return report;
    }

    report.exec = exec(&session);
    report.sftp_entries = sftp_ls(&session, sftp_path);
    report.sniff = sniff_path.map(|path| sniff_remote(&session, path));

    report
}

fn connect(host: &str, port: u16) -> Result<Session, String> {
    let address = format!("{host}:{port}");
    let socket = address
        .parse()
        .map_err(|_| "アドレスを解決できません".to_string())
        .and_then(|addr| {
            TcpStream::connect_timeout(&addr, TIMEOUT).map_err(|error| error.to_string())
        })
        .or_else(|_| TcpStream::connect(&address).map_err(|error| error.to_string()))?;

    let mut session = Session::new().map_err(|error| error.to_string())?;
    session.set_timeout(TIMEOUT.as_millis() as u32);
    session.set_tcp_stream(socket);
    session.handshake().map_err(|error| error.to_string())?;
    Ok(session)
}

fn fingerprint(session: &Session) -> Option<String> {
    let raw = session.host_key_hash(HashType::Sha256)?;
    Some(format!("SHA256:{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(raw)))
}

fn negotiated(session: &Session) -> Vec<(&'static str, String)> {
    [
        ("鍵交換", MethodType::Kex),
        ("ホスト鍵", MethodType::HostKey),
        ("暗号(c2s)", MethodType::CryptCs),
        ("MAC(c2s)", MethodType::MacCs),
        ("圧縮(c2s)", MethodType::CompCs),
    ]
    .into_iter()
    .filter_map(|(label, kind)| session.methods(kind).map(|value| (label, value.to_string())))
    .collect()
}

fn authenticate(session: &Session, user: &str, auth: &Auth) -> Result<(), String> {
    match auth {
        Auth::Agent => session.userauth_agent(user).map_err(|error| error.to_string()),
        Auth::Key { path, passphrase } => session
            .userauth_pubkey_file(user, None, Path::new(path), passphrase.as_deref())
            .map_err(|error| error.to_string()),
    }?;

    if session.authenticated() { Ok(()) } else { Err("認証が通りませんでした".into()) }
}

fn exec(session: &Session) -> Result<String, String> {
    let mut channel = session.channel_session().map_err(|error| error.to_string())?;
    channel.exec(&format!("echo {EXEC_MARKER}")).map_err(|error| error.to_string())?;

    let mut output = String::new();
    channel.read_to_string(&mut output).map_err(|error| error.to_string())?;
    let _ = channel.wait_close();

    let trimmed = output.trim().to_string();
    if trimmed == EXEC_MARKER {
        Ok(trimmed)
    } else {
        // 中身をそのまま出さない。想定外の出力に接続先が混ざりうる。
        Err(format!("想定と違う出力（{} バイト）", output.len()))
    }
}

fn sftp_ls(session: &Session, path: &str) -> Result<usize, String> {
    let sftp = session.sftp().map_err(|error| error.to_string())?;
    let entries = sftp.readdir(Path::new(path)).map_err(|error| error.to_string())?;
    Ok(entries.len())
}

fn sniff_remote(session: &Session, path: &str) -> Result<sniff::Sniff, String> {
    let sftp = session.sftp().map_err(|error| error.to_string())?;
    let mut file = sftp.open(Path::new(path)).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    Ok(sniff::sniff(&bytes))
}
