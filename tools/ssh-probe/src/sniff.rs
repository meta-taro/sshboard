//! 受け取ったバイト列の文字コードを見る。
//!
//! **なぜ要るか**: 対象は従来型の国内サーバーで、ログや設定ファイルが
//! EUC-JP / Shift_JIS の可能性がある。`xterm.js` は UTF-8 前提であり、
//! `read_file` で AI に渡す文字列も同じ問題を踏む。
//! **文字化けした設定ファイルを AI に読ませても意味がない。**
//!
//! **中身は返しません。**判定結果と統計だけを返します。
//! ファイルの中身には接続先や秘密情報が入りうるので、出力へ流さない（PRD §8）。

/// 判定の結果。**本文を持たない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sniff {
    pub bytes: usize,
    /// 0x80 以上のバイト数。0 なら ASCII だけで、判定するまでもない。
    pub non_ascii_bytes: usize,
    pub is_valid_utf8: bool,
    /// chardetng の推定（`UTF-8` / `Shift_JIS` / `EUC-JP` など）。
    pub detected: &'static str,
}

/// バイト列を見て、符号化方式を推定する。
pub fn sniff(bytes: &[u8]) -> Sniff {
    let non_ascii_bytes = bytes.iter().filter(|byte| **byte >= 0x80).count();
    let is_valid_utf8 = std::str::from_utf8(bytes).is_ok();

    // ISO-2022-JP を許可する。ブラウザ向けの既定では禁止だが、
    // ここが読むのはサーバー上のログと設定ファイルで、スクリプトは走らない。
    // Tera Term の漢字コード一覧にも JIS が入っている。
    let mut detector =
        chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let detected = detector.guess(None, chardetng::Utf8Detection::Allow).name();

    Sniff { bytes: bytes.len(), non_ascii_bytes, is_valid_utf8, detected }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAPANESE: &str = "設定ファイルの読み込みに失敗しました。ログを確認してください。\
                            メールの配送キューが滞留しています。ディスクの空き容量を確認します。";

    #[test]
    fn pure_ascii_has_no_high_bytes_and_is_valid_utf8() {
        // Arrange
        let bytes = b"postfix/smtpd[1234]: connect from unknown\n";

        // Act
        let result = sniff(bytes);

        // Assert
        assert_eq!(result.non_ascii_bytes, 0);
        assert!(result.is_valid_utf8);
        assert_eq!(result.bytes, bytes.len());
    }

    #[test]
    fn utf8_japanese_is_valid_utf8() {
        // Act
        let result = sniff(JAPANESE.as_bytes());

        // Assert
        assert!(result.is_valid_utf8);
        assert!(result.non_ascii_bytes > 0);
        assert_eq!(result.detected, "UTF-8");
    }

    #[test]
    fn shift_jis_is_not_valid_utf8_and_is_named() {
        // Arrange
        let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(JAPANESE);
        assert!(!had_errors, "テスト用の変換に失敗している");

        // Act
        let result = sniff(&encoded);

        // Assert
        assert!(!result.is_valid_utf8, "Shift_JIS が UTF-8 として通ってしまっている");
        assert_eq!(result.detected, "Shift_JIS");
    }

    #[test]
    fn euc_jp_is_not_valid_utf8_and_is_named() {
        // Arrange
        let (encoded, _, had_errors) = encoding_rs::EUC_JP.encode(JAPANESE);
        assert!(!had_errors, "テスト用の変換に失敗している");

        // Act
        let result = sniff(&encoded);

        // Assert
        assert!(!result.is_valid_utf8, "EUC-JP が UTF-8 として通ってしまっている");
        assert_eq!(result.detected, "EUC-JP");
    }

    #[test]
    fn iso_2022_jp_is_named_too() {
        // Tera Term の漢字コード一覧に JIS が入っている。メール系で出うる。
        // Arrange
        let (encoded, _, had_errors) = encoding_rs::ISO_2022_JP.encode(JAPANESE);
        assert!(!had_errors, "テスト用の変換に失敗している");

        // Act
        let result = sniff(&encoded);

        // Assert
        assert_eq!(result.detected, "ISO-2022-JP");
    }

    #[test]
    fn the_result_never_carries_the_content_itself() {
        // 中身にはホスト名やパスが入りうる。出力へ流さない（PRD §8）。
        let result = sniff(JAPANESE.as_bytes());
        let rendered = format!("{result:?}");

        assert!(!rendered.contains("設定"), "本文が結果に混ざっている: {rendered}");
    }
}
