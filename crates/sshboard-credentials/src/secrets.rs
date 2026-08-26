//! OS 資格情報ストア（Windows Credential Manager / macOS Keychain）への口。
//!
//! **この製品は秘密を持ちません。**置き場所は OS で、ここはその参照を扱うだけです（D11）。
//!
//! **`keyring` の罠に注意**（dbboard ADR-0033）:
//! 既定のバックエンドが無い状態だと **in-memory の mock** に解決され、
//! **書き込みは `Ok` を返すのに永続化されません。**
//! `Cargo.toml` の target ごとの feature を消さないこと。

/// 触れなかった理由。**秘密そのものをエラーに載せない。**
#[derive(Debug)]
pub enum SecretError {
    /// OS のストアが答えない。
    Store(String),
    /// その名前で保存されたものが無い。
    NotFound { reference: String },
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Store(detail) => write!(f, "OS の資格情報ストアが使えません: {detail}"),
            SecretError::NotFound { reference } => {
                write!(f, "`{reference}` に対応する保存がありません")
            }
        }
    }
}

impl std::error::Error for SecretError {}

/// OS の資格情報ストア。
///
/// `service` は OS 側での区分名。製品ごとに変えます。
pub struct SecretStore {
    service: String,
}

impl SecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    /// 取り出す。**無いことと、壊れていることを区別する。**
    pub fn get(&self, reference: &str) -> Result<String, SecretError> {
        match self.entry(reference)?.get_password() {
            Ok(secret) => Ok(secret),
            Err(keyring::Error::NoEntry) => Err(SecretError::NotFound {
                reference: reference.to_owned(),
            }),
            Err(error) => Err(SecretError::Store(error.to_string())),
        }
    }

    /// 入れる。**人が投入する経路です。**AI が秘密を作って入れるためのものではありません
    /// （product-baseline §14）。
    pub fn put(&self, reference: &str, secret: &str) -> Result<(), SecretError> {
        self.entry(reference)?
            .set_password(secret)
            .map_err(|error| SecretError::Store(error.to_string()))
    }

    /// 消す。無ければ無いと返す。
    pub fn delete(&self, reference: &str) -> Result<(), SecretError> {
        match self.entry(reference)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(SecretError::NotFound {
                reference: reference.to_owned(),
            }),
            Err(error) => Err(SecretError::Store(error.to_string())),
        }
    }

    fn entry(&self, reference: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(&self.service, reference)
            .map_err(|error| SecretError::Store(error.to_string()))
    }
}
