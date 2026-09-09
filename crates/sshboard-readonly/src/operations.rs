//! **状態を変える操作**を、人が列挙する一覧（D45 / Issue #17）。
//!
//! `readonly.toml` は**読み取り専用のまま触りません。**別ファイルにします。
//!
//! > **`readonly` という名前のファイルに書き込み操作が入ります。**
//! > 「約束はゲートではない」で排除したものが、名前だけのゲートとして戻ってきます
//!
//! **名前で区別がつくこと**が要点です。
//!
//! ## 崩していないもの
//!
//! **AI が渡せるのは id だけ**（D3）。`run` の文字列は**人が書いたもの**で、
//! AI が組み立てる余地はありません。**引数のスロットを作りません。**
//!
//! ## 製品が保証できないこと
//!
//! **「本当にその操作だけをするか」は検証できません。**走るのは人が書いた文字列です。
//! `readonly.toml` と同じで、**そこは人が読んで確かめる所**です。
//!
//! だから **`sudo` のパスワードは扱いません**（D46）。範囲は `sudoers.d` で
//! **OS に強制させます** —— アプリの約束ではなく。

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

/// **何をしても遠隔から踏ませないもの**（D47 の 6）。
///
/// **`operations.toml` に書いても弾きます。設定で有効にできません。**
/// dbboard が `GRANT` / `DROP` を恒久的に断っているのと同じ形です。
///
/// **AI が自分の檻を広げられない**ことが、この一覧の芯です。
const NEVER: [&str; 20] = [
    // 檻そのものを書き換える
    "sudoers",
    "visudo",
    "authorized_keys",
    // **鍵そのもの。**読み出す形も含めて弾きます ——
    // 実際にテストで `cat ~/.ssh/id_ed25519` が通り抜けました。
    ".ssh",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    // sshboard 自身の設定
    "connections.toml",
    "readonly.toml",
    "operations.toml",
    // **パスワードの入れ物。**`become = "ask"`（D48）を入れるまで、
    // root しか読めないものは**そもそも読めませんでした。**
    // 届くようになった以上、ここも塞ぎます。
    "/etc/shadow",
    "/etc/gshadow",
    // 利用者とロール
    "useradd",
    "usermod",
    "userdel",
    "groupadd",
    // 壊す
    "mkfs",
    "fdisk",
    "dd ",
    "rm -rf",
];

/// 1 件。**`run` は人が書いた文字列**で、AI は触れません。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Operation {
    /// AI が渡す識別子。**渡せるのはこれだけ。**
    pub id: String,
    /// 実際に打つもの。**人が書きます。**
    pub run: String,
    /// 人が読むための説明。**承認の画面に出ます。**
    pub description: String,
    /// 1 時間あたりの上限。**暴走しても、ここで止まります。**
    pub max_per_hour: u32,
}

/// 読めなかった理由。**空として扱わないため、種類を分けます。**
#[derive(Debug)]
pub enum OperationsError {
    /// 書き方が違う・足りない・重なっている。
    ///
    /// **空として扱いません。**扱うと「書いたのに断られる」になり、
    /// **原因が書き間違いだと誰も気づけません。**
    Malformed(String),
    /// **何をしても踏ませないもの**が書かれていた（D47 の 6）。
    NeverAllowed { id: String, matched: String },
    /// ファイルが読めない（在るのに読めない）。
    Unreadable(String),
}

impl std::fmt::Display for OperationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationsError::Malformed(detail) => {
                write!(f, "operations.toml を読めません: {detail}")
            }
            OperationsError::NeverAllowed { id, matched } => write!(
                f,
                "{id} は入れられません（{matched} を含みます）。\
                 **これは設定で有効にできません** —— 檻を広げる操作・利用者や鍵と\
                 パスワードの読み書き・壊す操作は、書いても弾きます"
            ),
            OperationsError::Unreadable(detail) => {
                write!(f, "operations.toml が読めません: {detail}")
            }
        }
    }
}

impl std::error::Error for OperationsError {}

/// 人が書いた一覧。**既定は空 ＝ 1 本も走りません。**
#[derive(Debug, Default, Deserialize)]
pub struct Operations {
    #[serde(default, rename = "operation")]
    operations: Vec<Operation>,
}

impl Operations {
    pub fn empty() -> Self {
        Self::default()
    }

    /// 読む。**通す前に、踏ませないものと書き間違いを弾きます。**
    pub fn parse(input: &str) -> Result<Self, OperationsError> {
        let parsed: Self =
            toml::from_str(input).map_err(|error| OperationsError::Malformed(error.to_string()))?;

        let mut seen = BTreeSet::new();
        for operation in &parsed.operations {
            if operation.id.trim().is_empty() {
                return Err(OperationsError::Malformed("id が空です".into()));
            }
            if operation.run.trim().is_empty() {
                return Err(OperationsError::Malformed(format!(
                    "{} に run がありません",
                    operation.id
                )));
            }
            // **上限の無い操作を書けない。**止まる所が要ります。
            if operation.max_per_hour == 0 {
                return Err(OperationsError::Malformed(format!(
                    "{} の max_per_hour が 0 です（1 以上にしてください）",
                    operation.id
                )));
            }
            // **同じ id が 2 つあると、どちらが走るのか人が読めません。**
            if !seen.insert(operation.id.clone()) {
                return Err(OperationsError::Malformed(format!(
                    "{} が 2 回書かれています",
                    operation.id
                )));
            }
            if let Some(matched) = never_allowed(&operation.run) {
                return Err(OperationsError::NeverAllowed {
                    id: operation.id.clone(),
                    matched: matched.to_owned(),
                });
            }
        }
        Ok(parsed)
    }

    /// 在れば読む。**無ければ空**（人がまだ書いていないだけ）。
    pub fn load_or_empty(path: &Path) -> Result<Self, OperationsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(OperationsError::Unreadable(error.to_string())),
        }
    }

    /// **id でだけ引けます。**
    pub fn get(&self, id: &str) -> Option<&Operation> {
        self.operations.iter().find(|held| held.id == id)
    }

    pub fn all(&self) -> &[Operation] {
        &self.operations
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// **書いても弾くもの**に当たっていれば、当たった語を返す。
fn never_allowed(run: &str) -> Option<&'static str> {
    let lowered = run.to_lowercase();
    NEVER.iter().copied().find(|word| lowered.contains(word))
}
