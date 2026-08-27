//! サーバーが**最初に出してくる** SSH_MSG_KEXINIT を、ライブラリを通さずに読む。
//!
//! **なぜ自前で読むか**: 002 の問いは「古い鍵交換方式・暗号方式が残っているか」。
//! ライブラリ経由だと「繋がった / 繋がらない」しか分からず、
//! **サーバーが何を提示しているのか**が見えない。ここが見えないと回避策も出せない。
//!
//! 読むだけで、鍵交換はしない。**接続先の情報は一切ここに残さない。**

/// サーバーが提示した方式の一覧。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerOffer {
    pub kex_algorithms: Vec<String>,
    pub host_key_algorithms: Vec<String>,
    pub encryption_client_to_server: Vec<String>,
    pub encryption_server_to_client: Vec<String>,
    pub mac_client_to_server: Vec<String>,
    pub mac_server_to_client: Vec<String>,
    pub compression_client_to_server: Vec<String>,
    pub compression_server_to_client: Vec<String>,
}

/// 読めなかった理由。**握り潰さない。**
#[derive(Debug, PartialEq, Eq)]
pub enum KexInitError {
    /// 先頭が SSH_MSG_KEXINIT ではない。
    NotKexInit { first_byte: u8 },
    /// 途中で尽きた。
    Truncated { at: usize },
    /// 名前リストが UTF-8 でない。
    NotUtf8 { list_index: usize },
}

impl std::fmt::Display for KexInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KexInitError::NotKexInit { first_byte } => {
                write!(
                    f,
                    "SSH_MSG_KEXINIT ではありません（先頭バイト {first_byte}）"
                )
            }
            KexInitError::Truncated { at } => write!(f, "{at} バイト目で尽きました"),
            KexInitError::NotUtf8 { list_index } => {
                write!(f, "{list_index} 番目の名前リストが UTF-8 ではありません")
            }
        }
    }
}

impl std::error::Error for KexInitError {}

/// SSH_MSG_KEXINIT のメッセージ番号（RFC 4253 §12）。
pub const SSH_MSG_KEXINIT: u8 = 20;

/// KEXINIT の payload を読む。
///
/// payload の形（RFC 4253 §7.1）:
/// `byte(20) / cookie(16) / name-list × 10 / boolean(1) / uint32(0)`
pub fn parse(payload: &[u8]) -> Result<ServerOffer, KexInitError> {
    let first = *payload.first().ok_or(KexInitError::Truncated { at: 0 })?;
    if first != SSH_MSG_KEXINIT {
        return Err(KexInitError::NotKexInit { first_byte: first });
    }

    // byte(1) + cookie(16) を飛ばして、名前リストが 10 本続く。
    let mut at = 1 + 16;
    let mut lists: Vec<Vec<String>> = Vec::with_capacity(NAME_LIST_COUNT);
    for index in 0..NAME_LIST_COUNT {
        let (list, next) = read_name_list(payload, at, index)?;
        lists.push(list);
        at = next;
    }

    // boolean(1) + uint32(4)。ここが無いなら途中で切れている。
    if payload.len() < at + 5 {
        return Err(KexInitError::Truncated { at });
    }

    let mut lists = lists.into_iter();
    let mut next = || {
        lists
            .next()
            .expect("10 本読んだことは上のループが保証している")
    };

    Ok(ServerOffer {
        kex_algorithms: next(),
        host_key_algorithms: next(),
        encryption_client_to_server: next(),
        encryption_server_to_client: next(),
        mac_client_to_server: next(),
        mac_server_to_client: next(),
        compression_client_to_server: next(),
        compression_server_to_client: next(),
    })
}

/// KEXINIT に並ぶ名前リストの本数（言語の 2 本を含む）。
const NAME_LIST_COUNT: usize = 10;

/// `uint32(長さ) + カンマ区切りの ASCII` を 1 本読み、次の位置を返す。
fn read_name_list(
    bytes: &[u8],
    at: usize,
    index: usize,
) -> Result<(Vec<String>, usize), KexInitError> {
    let after_len = at + 4;
    let raw_len = bytes
        .get(at..after_len)
        .ok_or(KexInitError::Truncated { at })?;
    let len = u32::from_be_bytes(raw_len.try_into().expect("4 バイト取れている")) as usize;

    let end = after_len + len;
    let raw = bytes
        .get(after_len..end)
        .ok_or(KexInitError::Truncated { at: after_len })?;
    let text = std::str::from_utf8(raw).map_err(|_| KexInitError::NotUtf8 { list_index: index })?;

    // 空リストは「空の Vec」。ここで `[""]` にすると、出力が嘘になる。
    let list = if text.is_empty() {
        Vec::new()
    } else {
        text.split(',').map(str::to_owned).collect()
    };

    Ok((list, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用に KEXINIT の payload を組み立てる。
    fn payload_with(lists: &[&str]) -> Vec<u8> {
        let mut out = vec![SSH_MSG_KEXINIT];
        out.extend_from_slice(&[0u8; 16]); // cookie
        for list in lists {
            out.extend_from_slice(&(list.len() as u32).to_be_bytes());
            out.extend_from_slice(list.as_bytes());
        }
        out.push(0); // first_kex_packet_follows
        out.extend_from_slice(&0u32.to_be_bytes()); // reserved
        out
    }

    fn ten_lists() -> Vec<&'static str> {
        vec![
            "diffie-hellman-group14-sha1,curve25519-sha256",
            "ssh-rsa,rsa-sha2-512",
            "aes128-ctr,3des-cbc",
            "aes128-ctr",
            "hmac-sha1,hmac-sha2-256",
            "hmac-sha1",
            "none,zlib@openssh.com",
            "none",
            "",
            "",
        ]
    }

    #[test]
    fn parses_every_name_list_the_server_offers() {
        // Arrange
        let payload = payload_with(&ten_lists());

        // Act
        let offer = parse(&payload).expect("読めない");

        // Assert
        assert_eq!(
            offer.kex_algorithms,
            vec!["diffie-hellman-group14-sha1", "curve25519-sha256"]
        );
        assert_eq!(offer.host_key_algorithms, vec!["ssh-rsa", "rsa-sha2-512"]);
        assert_eq!(
            offer.encryption_client_to_server,
            vec!["aes128-ctr", "3des-cbc"]
        );
        assert_eq!(offer.mac_server_to_client, vec!["hmac-sha1"]);
        assert_eq!(
            offer.compression_client_to_server,
            vec!["none", "zlib@openssh.com"]
        );
    }

    #[test]
    fn an_empty_name_list_becomes_an_empty_vec_not_a_list_with_one_empty_string() {
        // languages_* はたいてい空。ここで "" が 1 件入ると、出力が嘘になる。
        let offer = parse(&payload_with(&ten_lists())).expect("読めない");

        assert!(offer
            .compression_server_to_client
            .contains(&"none".to_string()));
    }

    #[test]
    fn a_message_that_is_not_kexinit_is_rejected() {
        // Arrange — 21 は SSH_MSG_NEWKEYS
        let mut payload = payload_with(&ten_lists());
        payload[0] = 21;

        // Act & Assert
        assert_eq!(
            parse(&payload),
            Err(KexInitError::NotKexInit { first_byte: 21 })
        );
    }

    #[test]
    fn a_truncated_payload_is_reported_instead_of_panicking() {
        // Arrange
        let payload = payload_with(&ten_lists());
        let cut = &payload[..30];

        // Act
        let result = parse(cut);

        // Assert
        assert!(
            matches!(result, Err(KexInitError::Truncated { .. })),
            "実際: {result:?}"
        );
    }
}
