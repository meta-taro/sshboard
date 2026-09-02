//! 接続情報を 1 つのファイルにして渡す（D18）。**パスフレーズで暗号化する。**
//!
//! `connections.toml` は、**それ単体では別のマシンで役に立ちません。**
//! 中身は OS ストアの参照名だけで（D11）、参照先が相手のマシンに無いためです。
//!
//! **実務で一番手間なのがここ**でした。IP はフォルダ名、ログイン ID は別の表、
//! 鍵は `.ppk`、パスフレーズはさらに別。**揃えること自体が仕事になっています。**
//!
//! ## 承知のうえで受け入れているリスク（D18）
//!
//! **このファイル 1 つで、書き出した相手のサーバー全部に入れます。**
//! それでも配るのは、「安全のために分散させる」に実利が無いからです —
//! PC が取られた時点で終わりであり、**接続時にはどのみち 1 箇所へ集約されます。**
//! 手元で散らしても攻撃者の手間が増えるだけで、守れてはいません。
//!
//! **パスフレーズはファイルと同じ経路で送らないこと**（D18）。
//!
//! ## 暗号は自前で書きません
//!
//! `age`（scrypt で鍵導出 ＋ ChaCha20-Poly1305 の認証付き暗号）に任せます。
//! **中身を 1 バイト書き換えたら開かない**のは、その認証のためです。

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use sshboard_connections::Connections;
use zeroize::Zeroize;

/// このビルドが読み書きする中身の版。
///
/// `connections.toml` の版とも `age` の版とも別です。
/// **中身の形を変えたらここを上げ、移行を明示的に書くこと。**
pub const BUNDLE_VERSION: u32 = 1;

/// 書き出すときに受け付ける最短のパスフレーズ。
///
/// **強さを測る物差しではありません。**空や打ち間違いを弾くための下限です。
/// 読み込む側では見ません（他の道具が作ったものも開けるように）。
pub const MIN_PASSPHRASE_LEN: usize = 8;

/// 拡張子。**中身は `age` の暗号文。**
pub const BUNDLE_EXTENSION: &str = "sshbx";

/// 暗号化する前の中身。**接続の一覧と、それが参照している秘密。**
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundlePayload {
    pub version: u32,
    /// `connections.toml` に入るのと同じもの。**秘密は入っていません**（参照名だけ）。
    pub connections: Connections,
    /// 参照名 → 秘密そのもの。**ここだけが本物の秘密です。**
    ///
    /// 並び順が決まる `BTreeMap` を使います。**同じ中身なら同じ並びになる**方が、
    /// 試験の検体が安定します。
    pub secrets: std::collections::BTreeMap<String, String>,
}

impl BundlePayload {
    #[must_use]
    pub fn new(
        connections: Connections,
        secrets: std::collections::BTreeMap<String, String>,
    ) -> Self {
        Self {
            version: BUNDLE_VERSION,
            connections,
            secrets,
        }
    }
}

/// **`{:?}` から秘密を消す。**ログにもパニックにも出る入口なので、
/// 値は出しません。参照名は秘密ではなく、追うのに要るので残します。
impl std::fmt::Debug for BundlePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BundlePayload")
            .field("version", &self.version)
            .field("connections", &self.connections)
            .field(
                "secrets",
                &format_args!("<{} 件・伏せています>", self.secrets.len()),
            )
            .finish()
    }
}

/// 捨てるときに秘密を消す。
///
/// 暗号化のときの中間バッファはその場で消していますが、
/// **この型はそれより長く生きます**（暗号化の前に組み立て、復号のあとに返す）。
/// 消さないと、解放済みのヒープに秘密が残ります（コアダンプや swap から拾える）。
impl Drop for BundlePayload {
    fn drop(&mut self) {
        for value in self.secrets.values_mut() {
            value.zeroize();
        }
    }
}

/// 失敗の種類。**人の次の一手が変わる所で分けています。**
#[derive(Debug)]
pub enum BundleError {
    /// パスフレーズが短すぎる（空を含む）。**書き出す前に断ります。**
    WeakPassphrase,
    /// 中身を JSON にできなかった。**実装の誤りでしか起きません。**
    Serialize(serde_json::Error),
    /// パスフレーズが合わない。**打ち間違いなら打ち直せばよい。**
    IncorrectPassphrase,
    /// ファイルが壊れている・途中で切れている・書き換えられている。
    /// **貰い直すしかありません。**
    Corrupt,
    /// このビルドが知らない版。
    UnsupportedVersion(u32),
    /// 開けたが、中身が sshboard のものではない。
    Parse(serde_json::Error),
    /// 読み書きの失敗。**メモリ上の処理なので、実際にはまず起きません。**
    Io(std::io::Error),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeakPassphrase => write!(
                f,
                "パスフレーズが短すぎます（{MIN_PASSPHRASE_LEN} 文字以上）"
            ),
            Self::Serialize(error) => write!(f, "中身を組み立てられません: {error}"),
            Self::IncorrectPassphrase => write!(f, "パスフレーズが違います"),
            Self::Corrupt => write!(f, "ファイルが壊れているか、sshboard のものではありません"),
            Self::UnsupportedVersion(version) => {
                write!(f, "この版のファイルは読めません（版 {version}）")
            }
            Self::Parse(error) => write!(f, "中身が sshboard のものではありません: {error}"),
            Self::Io(error) => write!(f, "読み書きに失敗しました: {error}"),
        }
    }
}

impl std::error::Error for BundleError {}

/// 書き出す。返るのは `.sshbx` へそのまま書ける中身。
///
/// # Errors
///
/// パスフレーズが短い・組み立てに失敗した・書き込みに失敗した場合。
pub fn encrypt_bundle(payload: &BundlePayload, passphrase: &str) -> Result<Vec<u8>, BundleError> {
    validate_passphrase(passphrase)?;

    let _kdf = kdf_guard();

    let mut plaintext = serde_json::to_vec(payload).map_err(BundleError::Serialize)?;

    let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
        passphrase.to_owned(),
    ));
    let mut encrypted = Vec::new();
    let result = (|| {
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&plaintext)?;
        writer.finish()?;
        Ok(())
    })()
    .map_err(BundleError::Io);

    // **成否によらず消す。**ここは一瞬でも全部の秘密を持っていた場所です。
    plaintext.zeroize();
    result?;

    Ok(encrypted)
}

/// 読み戻す。
///
/// # Errors
///
/// パスフレーズが違う・壊れている・版が読めない・中身が別物の場合。
pub fn decrypt_bundle(blob: &[u8], passphrase: &str) -> Result<BundlePayload, BundleError> {
    let _kdf = kdf_guard();

    // 1. 見出しの段。**ここで落ちるなら、パスフレーズはまだ使っていません** —
    //    そもそも `age` のファイルではない、ということです。
    let decryptor = age::Decryptor::new(blob).map_err(|_| BundleError::Corrupt)?;

    // 2. 鍵を開ける段。**ここでパスフレーズを使います。**
    //    `age` は「違うパスフレーズ」と「壊れた鍵の欄」を区別できません
    //    （どちらも同じ検査に落ちる）。**打ち間違いの方が圧倒的に多い**ので、
    //    そちらとして伝えます。
    let identity =
        age::scrypt::Identity::new(age::secrecy::SecretString::from(passphrase.to_owned()));
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| BundleError::IncorrectPassphrase)?;

    // 3. 中身の段。**鍵はもう開いている**ので、ここでの失敗は改竄です。
    let mut plaintext = Vec::new();
    let outcome = match reader.read_to_end(&mut plaintext) {
        Ok(_) => parse_payload(&plaintext),
        Err(_) => Err(BundleError::Corrupt),
    };
    // **解いた JSON も消す。**取り出したあとも平文で残っています。
    plaintext.zeroize();
    outcome
}

/// 書き出す前にパスフレーズだけ見る。**画面が、暗号化を始める前に断れるように。**
///
/// # Errors
///
/// 短すぎる場合。
pub fn validate_passphrase(passphrase: &str) -> Result<(), BundleError> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(BundleError::WeakPassphrase);
    }
    Ok(())
}

fn parse_payload(plaintext: &[u8]) -> Result<BundlePayload, BundleError> {
    let payload: BundlePayload = serde_json::from_slice(plaintext).map_err(BundleError::Parse)?;
    if payload.version != BUNDLE_VERSION {
        return Err(BundleError::UnsupportedVersion(payload.version));
    }
    Ok(payload)
}

/// **試験のときだけ**鍵導出を 1 本ずつにする。
///
/// `age` は scrypt（log_n = 18）で鍵を作るので、1 回の書き出し／読み戻しが
/// **256 MiB の連続領域を一度に要求します。**試験は CPU の数だけ並列に走るため、
/// このファイルの試験が同時に動くと数 GB を同じ瞬間に要求します。
/// 足りないと **試験のバイナリごと落ち**、Windows では
/// `STATUS_STACK_BUFFER_OVERRUN`（0xc0000409）として出ます —
/// **メモリ安全性の不具合に見えますが、違います。**
/// 空きメモリ次第なので、**同じコードが混んだ機械で落ち、空いた機械で通ります。**
///
/// 製品では錠を取りません。書き出しも取り込みも**人が 1 回押す操作**であって、
/// 20 個同時には起きません。
#[cfg(test)]
fn kdf_guard() -> std::sync::MutexGuard<'static, ()> {
    static KDF: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // 1 つの試験が落ちても、後続を巻き込まない。
    KDF.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(not(test))]
struct KdfGuard;

#[cfg(not(test))]
fn kdf_guard() -> KdfGuard {
    KdfGuard
}
