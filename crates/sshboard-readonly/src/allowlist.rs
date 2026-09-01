//! `readonly.toml` の読み込み。
//!
//! **無いことは異常ではありません。**まだ 1 本も許可していないだけです。
//! ただし**読めないことは異常です。**空として扱うと、人は
//! 「許可したはずなのに断られる」に遭い、原因がここだと気づけません。

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::command::ReadonlyCommand;

/// いまの書式。**上げたら、読めない版を黙って捨てない。**
pub const CURRENT_VERSION: u32 = 1;

/// 読めなかった理由。**握り潰さない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowlistError {
    /// 書式が読めない。
    Malformed { detail: String },
    /// 知らない版。**捨てずに止める。**
    UnknownVersion { found: u32 },
    /// 同じ識別子が 2 つ。**どちらを走らせるか決められない。**
    DuplicateId { id: String },
    /// 識別子が空。
    EmptyId,
    /// 走る中身が空。**引けるのに何も起きない項目を残さない。**
    EmptyCommand { id: String },
    /// 走る中身に改行や制御文字が入っている。
    /// **人が読んだ 1 行と、実際に走る中身がずれる。**
    ControlCharacter { id: String },
    /// ファイルを触れない。
    Io { detail: String },
}

impl std::fmt::Display for AllowlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllowlistError::Malformed { detail } => {
                write!(f, "コマンドの許可リストを読めません: {detail}")
            }
            AllowlistError::UnknownVersion { found } => {
                write!(f, "知らない版の許可リストです（version = {found}）")
            }
            AllowlistError::DuplicateId { id } => {
                write!(f, "許可リストの識別子が重複しています: {id}")
            }
            AllowlistError::EmptyId => write!(f, "識別子が空の項目が許可リストにあります"),
            AllowlistError::EmptyCommand { id } => {
                write!(f, "`{id}` に走らせる中身がありません")
            }
            AllowlistError::ControlCharacter { id } => {
                write!(f, "`{id}` の中身に改行か制御文字が入っています")
            }
            AllowlistError::Io { detail } => {
                write!(f, "コマンドの許可リストを触れません: {detail}")
            }
        }
    }
}

impl std::error::Error for AllowlistError {}

/// AI が呼べるコマンドの一覧。**人が書きます。**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allowlist {
    pub version: u32,
    /// **並びは人が書いた通りに保ちます。**
    /// 毎回並べ替えると、人が差分で確かめられません。
    #[serde(default, rename = "command")]
    commands: Vec<ReadonlyCommand>,
}

impl Allowlist {
    /// **既定。1 本も許可していない状態**（D3 追記）。
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            commands: Vec::new(),
        }
    }

    /// TOML から読む。**版・重複・空をここで弾く。**
    pub fn parse(input: &str) -> Result<Self, AllowlistError> {
        let parsed: Self = toml::from_str(input).map_err(|error| AllowlistError::Malformed {
            detail: error.to_string(),
        })?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// ファイルから読む。**無ければ空を返す。ファイルは作らない。**
    pub fn load_or_empty(path: &Path) -> Result<Self, AllowlistError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            // まだ 1 本も許可していないだけ。**異常ではない。**
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(AllowlistError::Io {
                detail: error.to_string(),
            }),
        }
    }

    /// 許可された 1 本を引く。**無ければ `None`**（呼ぶ側が断って記録します）。
    pub fn get(&self, id: &str) -> Option<&ReadonlyCommand> {
        self.commands.iter().find(|command| command.id == id)
    }

    /// 許可された全部。**人が書いた並びのまま。**
    pub fn commands(&self) -> &[ReadonlyCommand] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn validate(&self) -> Result<(), AllowlistError> {
        if self.version != CURRENT_VERSION {
            return Err(AllowlistError::UnknownVersion {
                found: self.version,
            });
        }

        let mut seen = HashSet::new();
        for command in &self.commands {
            if command.id.trim().is_empty() {
                return Err(AllowlistError::EmptyId);
            }
            if !seen.insert(command.id.as_str()) {
                return Err(AllowlistError::DuplicateId {
                    id: command.id.clone(),
                });
            }
            if command.run.trim().is_empty() {
                return Err(AllowlistError::EmptyCommand {
                    id: command.id.clone(),
                });
            }
            // **1 項目に 2 行入れさせない。**許可リストは、人が目で追えることに意味がある。
            if command.run.chars().any(char::is_control) {
                return Err(AllowlistError::ControlCharacter {
                    id: command.id.clone(),
                });
            }
        }

        Ok(())
    }
}
