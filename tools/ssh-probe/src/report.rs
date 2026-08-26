//! 探り棒の出力。**接続先を書かない。**
//!
//! ここに載せてよいのは「繋がったか」「どの方式か」だけ（PRD §8・Issue 002）。
//! ディレクトリの中身やファイルの中身は**件数と判定だけ**にする。
//! 名前にはパスや利用者名が混ざる。

use crate::sniff::Sniff;

/// 片方のライブラリで試した結果。
#[derive(Debug)]
pub struct BackendReport {
    pub name: &'static str,
    /// サーバーのホスト鍵指紋。**人が known_hosts と突き合わせるために出す。**
    pub host_key_fingerprint: Option<String>,
    /// 実際に成立した方式（ライブラリが教えてくれる場合のみ）。
    pub negotiated: Vec<(&'static str, String)>,
    pub connected: Option<String>,
    pub authenticated: Option<String>,
    /// `exec` の結果。流すのはこちらが指定した固定文字列だけ。
    pub exec: Result<String, String>,
    /// `sftp` の `ls` で見えた件数。**名前は出さない。**
    pub sftp_entries: Result<usize, String>,
    /// 文字コードの判定。**本文は含まない。**
    pub sniff: Option<Result<Sniff, String>>,
}

impl BackendReport {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            host_key_fingerprint: None,
            negotiated: Vec::new(),
            connected: None,
            authenticated: None,
            exec: Err("試していません".into()),
            sftp_entries: Err("試していません".into()),
            sniff: None,
        }
    }

    pub fn render(&self) -> String {
        let mut out = format!("## {}\n", self.name);

        match (&self.connected, &self.authenticated) {
            (Some(error), _) => out.push_str(&format!("- 接続: 失敗 — {error}\n")),
            (None, Some(error)) => {
                out.push_str("- 接続: OK\n");
                out.push_str(&format!("- 認証: 失敗 — {error}\n"));
            }
            (None, None) => {
                out.push_str("- 接続: OK\n- 認証: OK\n");
            }
        }

        if let Some(fingerprint) = &self.host_key_fingerprint {
            out.push_str(&format!("- ホスト鍵: {fingerprint}\n"));
        }
        for (label, value) in &self.negotiated {
            out.push_str(&format!("- 成立した{label}: {value}\n"));
        }

        out.push_str(&match &self.exec {
            Ok(text) => format!("- exec: OK（{text}）\n"),
            Err(error) => format!("- exec: 失敗 — {error}\n"),
        });
        out.push_str(&match &self.sftp_entries {
            Ok(count) => format!("- sftp ls: OK（{count} 件・名前は出しません）\n"),
            Err(error) => format!("- sftp ls: 失敗 — {error}\n"),
        });

        if let Some(sniff) = &self.sniff {
            out.push_str(&match sniff {
                Ok(s) => format!(
                    "- 文字コード: {} / {} バイト中 非 ASCII {} / UTF-8 として妥当={}\n",
                    s.detected, s.bytes, s.non_ascii_bytes, s.is_valid_utf8
                ),
                Err(error) => format!("- 文字コード: 読めません — {error}\n"),
            });
        }

        out
    }
}
