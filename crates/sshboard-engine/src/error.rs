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
    /// 端末を別の側が握っている（D29）。
    ///
    /// **同時に触れるのは 1 人。**人と AI が交互に打つと、
    /// **どちらの意図でもない文字列**がシェルへ入る。
    ConsoleHeldByOther { holder: String },
    /// 端末がまだ開いていない。
    ConsoleNotOpen,
    /// 指した鍵を認証に使えない（公開鍵・鍵ではないファイル）。
    ///
    /// **形式の名前だけを持ちます。**鍵のパスは接続先の情報なので、
    /// 画面にも記録にも出しません（CLAUDE.md 禁止事項 4）。
    UnusableKey { id: String, format: String },
    /// **ホスト鍵を信用できない。**初見か、登録と食い違う。
    ///
    /// 文字列にせず**構造のまま**返す。画面が「この指紋で登録しますか」を
    /// 出せなければ、人はここで行き止まりになる（**実際になった**）。
    UntrustedHost {
        id: String,
        algorithm: String,
        fingerprint: String,
        /// 登録済みの指紋。**あるのに食い違っているなら、すり替えの疑い。**
        expected: Option<String>,
    },
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
            EngineError::ConsoleHeldByOther { holder } => write!(
                f,
                "端末は{holder}が握っています。**同時に触れるのは 1 人です**（D29）。\
                 人は画面の［取り返す］でいつでも取り返せます"
            ),
            EngineError::ConsoleNotOpen => write!(
                f,
                "端末がまだ開いていません。先に開いてから打ってください"
            ),
            EngineError::UnusableKey { id, format } => write!(
                f,
                "{id} に指定されたファイルは認証に使えません（{format} と読めました）。\
                 秘密鍵のファイルを指してください（`.pub` は公開鍵で、認証には使えません）"
            ),
            EngineError::UntrustedHost {
                algorithm,
                fingerprint,
                expected: Some(expected),
                ..
            } => write!(
                f,
                "ホスト鍵が登録と違います（{algorithm} / 見えたもの {fingerprint}・登録 {expected}）"
            ),
            EngineError::UntrustedHost {
                algorithm,
                fingerprint,
                expected: None,
                ..
            } => write!(
                f,
                "初めて見るホストです（{algorithm} / {fingerprint}）。確かめて登録してください"
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
