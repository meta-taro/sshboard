//! サーバーが最初に出す KEXINIT を、ライブラリを通さずに取りに行く。
//!
//! 鍵交換はしない。**バナーと KEXINIT を読んで、すぐ切る。**

use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::kexinit::{self, ServerOffer};

/// こちらが名乗る版。RFC 4253 §4.2 の形。
const OUR_BANNER: &str = "SSH-2.0-sshboard_probe_0.0.0\r\n";

/// 1 パケットの上限。KEXINIT は数百バイトで、これを超えるのは異常。
const MAX_PACKET: usize = 64 * 1024;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// サーバーの名乗りと、提示された方式。
#[derive(Debug)]
pub struct FirstContact {
    /// `SSH-2.0-OpenSSH_7.4` のような版文字列。**古さがここに出る。**
    pub banner: String,
    pub offer: ServerOffer,
}

pub async fn fetch(host: &str, port: u16) -> Result<FirstContact> {
    let mut stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .context("接続がタイムアウトしました")?
        .context("TCP で繋がりません")?;

    let banner = read_banner(&mut stream).await?;
    stream.write_all(OUR_BANNER.as_bytes()).await?;

    let payload = read_packet_payload(&mut stream).await?;
    let offer = kexinit::parse(&payload).context("KEXINIT を読めません")?;

    Ok(FirstContact { banner, offer })
}

/// `SSH-` で始まる行が来るまで読む。前置きの行はサーバーが出すことがある。
async fn read_banner(stream: &mut TcpStream) -> Result<String> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];

    for _ in 0..MAX_PACKET {
        stream.read_exact(&mut byte).await.context("バナーが読めません")?;
        if byte[0] == b'\n' {
            let text = String::from_utf8_lossy(&line).trim_end().to_string();
            if text.starts_with("SSH-") {
                return Ok(text);
            }
            line.clear();
            continue;
        }
        line.push(byte[0]);
    }

    bail!("バナーが見つかりません")
}

/// 暗号化前のバイナリパケットを 1 つ読む（RFC 4253 §6）。
async fn read_packet_payload(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).await.context("パケット長が読めません")?;
    let packet_len = u32::from_be_bytes(length) as usize;

    if packet_len == 0 || packet_len > MAX_PACKET {
        bail!("パケット長が異常です: {packet_len}");
    }

    let mut rest = vec![0u8; packet_len];
    stream.read_exact(&mut rest).await.context("パケット本体が読めません")?;

    let padding = *rest.first().context("パディング長が読めません")? as usize;
    if padding + 1 > rest.len() {
        bail!("パディング長がパケットを超えています");
    }

    Ok(rest[1..rest.len() - padding].to_vec())
}
