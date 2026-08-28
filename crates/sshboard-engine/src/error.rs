//! 断る理由。**「駄目でした」で終わらせない**（product-baseline §17）。

use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    /// まだどこにも繋がっていない。
    NotConnected,
    /// 別の接続が開いている。**黙って乗り換えない。**
    AlreadyConnected { id: String, name: String },
    /// 接続一覧に無い識別子。
    UnknownConnection(String),
    /// 接続一覧そのものが読めない。
    Connections(String),
    /// 鍵にパスフレーズが要る。**AI は受け取れない**（D14）。
    PassphraseNeeded { id: String },
    /// SSH 側の失敗。ホスト鍵の不一致もここに入る。
    Ssh(sshboard_ssh::SshError),
    /// ローカルのファイルが読めない・書けない。
    Local(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotConnected => write!(
                f,
                "まだサーバーに繋がっていません。sshboard の画面で接続を開いてください"
            ),
            EngineError::AlreadyConnected { id, name } => write!(
                f,
                "すでに {name}（{id}）へ繋がっています。\
                 先に切ってから繋ぎ直してください（同時に 2 本は張りません）"
            ),
            EngineError::UnknownConnection(id) => {
                write!(f, "{id} という接続は登録されていません")
            }
            EngineError::Connections(detail) => {
                write!(f, "接続一覧を読めません: {detail}")
            }
            EngineError::PassphraseNeeded { id } => write!(
                f,
                "{id} の鍵にはパスフレーズが要ります。\
                 sshboard の画面で人が入れてください（AI はパスフレーズを扱いません）"
            ),
            EngineError::Ssh(error) => write!(f, "{error}"),
            EngineError::Local(detail) => write!(f, "手元のファイルを扱えません: {detail}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<sshboard_ssh::SshError> for EngineError {
    fn from(error: sshboard_ssh::SshError) -> Self {
        EngineError::Ssh(error)
    }
}
