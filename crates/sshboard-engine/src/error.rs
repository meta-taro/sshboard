//! 断る理由。**「駄目でした」で終わらせない**（product-baseline §17）。

use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    /// まだどこにも繋がっていない。
    NotConnected,
    /// **開いているのに、宛先が決まっていない**（Issue #8）。
    ///
    /// `NotConnected` と分けます。**同じ「繋がっていません」で括ると、
    /// 人に効かない手順を案内してしまう**からです ——
    /// この状態で「画面で接続を開いてください」と言われても、
    /// **すでに開いています。**何本開いていても、押す所が分からない。
    ///
    /// **勝手に 1 本選んで繋ぎ直しません。**この製品は
    /// 「どのサーバーを触っているか、人に見えていること」を安全の根拠に
    /// しています（PRD §4-1）。**推測した相手へコマンドを流すのが一番危険**です。
    NoTarget { open: usize },
    /// **指した接続が開いていない**（Issue #8）。
    ///
    /// `NotConnected` と分けます。**画面がタブを押したのに開いていない**なら、
    /// 画面が持っている一覧が実態とずれています —— 「接続を開いてください」では
    /// **どのタブが古いのか分かりません。**
    ///
    /// この 2 つ（[`NoTarget`](EngineError::NoTarget) と こちら）を分けたのは、
    /// **報告された文言からどちらの道か判別できなかった**からです。
    /// 同じ「まだサーバーに繋がっていません」を 2 か所が返していました。
    NotOpen { id: String },
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
    /// **AI が端末を握るには、人の許可が要る**（D42）。
    ///
    /// 頼んだことは記録に残り、画面へ出ます。**人が答えるまで握れません。**
    /// これは故障ではなく、**人に気づかせるための一手**です。
    ConsoleApprovalNeeded,
    /// 端末は**別の接続**で開いている（D29 ＋ D25）。
    ///
    /// **黙って乗り換えない。**タブを移したつもりで、打鍵が前のサーバーへ
    /// 行き続けるのが一番危ない。
    ConsoleOnOtherConnection { id: String },
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
    /// **許可リストに無いコマンド**（D3）。
    ///
    /// AI が渡せるのは人が書いた一覧の中の識別子だけです。
    /// **これは故障ではありません。**足りていないものが 1 つ見つかった、という報せで、
    /// 断った事実は `readonly-refused.log` に残ります（足す判断は人）。
    NotAllowed { id: String },
    /// 許可リストそのものが読めない。
    ///
    /// **空として扱いません。**扱うと「許可したのに断られる」になり、
    /// 原因がファイルの書き間違いだと誰も気づけません。
    Allowlist(String),
    /// 用途別ツールに渡す値が足りない・使えない。
    ///
    /// **空のまま投げると、別のものが返ってきます**（`systemctl status` は
    /// 名前が無いと全ユニットを吐く）。サーバーへ行く前に断ります。
    BadArgument(String),
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
            EngineError::NotOpen { id } => write!(
                f,
                "{id} は開いていません。画面のタブが実態とずれています——\
                 いったん切って繋ぎ直すか、この文言をそのまま貼って報告してください（Issue #8）"
            ),
            EngineError::NoTarget { open } => write!(
                f,
                "{open} 本つながっていますが、操作の宛先が決まっていません。\
                 sshboard の画面でタブを押して、どのサーバーへ送るか選んでください"
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
                 **sshboard の画面に問いを出しました。**人が入れたら、もう一度繋いでください。\
                 画面が見当たらないときは、人に sshboard を前面へ出して\
                 ［接続］を押してもらってください（AI はパスフレーズを扱いません・D14）"
            ),
            EngineError::ConsoleHeldByOther { holder } => write!(
                f,
                "端末は{holder}が握っています。**同時に触れるのは 1 人です**（D29）。\
                 人は画面の［取り返す］でいつでも取り返せます"
            ),
            EngineError::ConsoleApprovalNeeded => write!(
                f,
                "端末を使ってよいか、いま人に尋ねています。\
                 **人が画面で許可するまで握れません**（D42）。\
                 少し待ってから開き直してください"
            ),
            EngineError::ConsoleNotOpen => write!(
                f,
                "端末がまだ開いていません。先に開いてから打ってください"
            ),
            EngineError::ConsoleOnOtherConnection { id } => write!(
                f,
                "端末は {id} で開いています。**別の接続では開き直しません**（打鍵が\
                 どちらへ行くのか分からなくなるため）。先に［止める］を押してください"
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
            EngineError::NotAllowed { id } => write!(
                f,
                "`{id}` は許可リストにありません。\
                 sshboard が AI に走らせるのは、人が readonly.toml へ書いたものだけです（D3）。\
                 断ったことは記録に残りました。**足すかどうかは人が決めます**"
            ),
            EngineError::Allowlist(detail) => write!(
                f,
                "コマンドの許可リストを読めません: {detail}。\
                 読めない一覧を空として扱わないので、いまは 1 本も走りません"
            ),
            EngineError::BadArgument(detail) => write!(f, "{detail}"),
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
