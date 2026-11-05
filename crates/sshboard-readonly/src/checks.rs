//! **点検表**（D72）。人が書く「許可リストの id の並び＋決まった形の基準」。
//!
//! ## 崩していないもの
//!
//! **手順に文字列を書く欄はありません。**手順が指せるのは `readonly.toml` の id だけで、
//! 走るのは人がそこへ書いた文字列です（D3）。知らない欄は読めない表として断ります。
//!
//! ## 既定は空
//!
//! 製品は点検を 1 本も持ちません。何を見るかは、その人のサーバーが決めます。

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::judge::Expect;

/// 表の版。**上げたら、古い版を読む道を残すか決めること。**
pub const CHECKS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChecksError {
    Malformed { detail: String },
    UnknownVersion { found: u32 },
    EmptyId,
    DuplicateId { id: String },
    NoSteps { id: String },
    EmptyStep { id: String },
    Io { detail: String },
}

impl std::fmt::Display for ChecksError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { detail } => write!(f, "checks.toml を読めません: {detail}"),
            Self::UnknownVersion { found } => {
                write!(f, "checks.toml の version = {found} は知りません（{CHECKS_VERSION} だけ読めます）")
            }
            Self::EmptyId => write!(f, "checks.toml に id の空な点検があります"),
            Self::DuplicateId { id } => write!(f, "checks.toml に点検 `{id}` が 2 つあります"),
            Self::NoSteps { id } => write!(f, "点検 `{id}` に手順が 1 つもありません"),
            Self::EmptyStep { id } => write!(f, "点検 `{id}` に readonly の空な手順があります"),
            Self::Io { detail } => write!(f, "checks.toml を開けません: {detail}"),
        }
    }
}

impl std::error::Error for ChecksError {}

/// 手順 1 つ。**`readonly.toml` の id を指すだけ。**
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// `readonly.toml` の id。
    pub readonly: String,
    /// 人に見せる名前。無ければ id を出します。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(flatten)]
    pub expect: Expect,
}

/// 点検 1 つ。手順は**書いた順に**回します。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, rename = "step")]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub version: u32,
    #[serde(default, rename = "check")]
    checks: Vec<Check>,
}

impl Checks {
    /// **既定。1 本も無い。**
    pub fn empty() -> Self {
        Self {
            version: CHECKS_VERSION,
            checks: Vec::new(),
        }
    }

    /// TOML から読む。**読めない表は空として扱わない。**
    pub fn parse(input: &str) -> Result<Self, ChecksError> {
        let parsed: Self = toml::from_str(input).map_err(|error| ChecksError::Malformed {
            detail: error.to_string(),
        })?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// ファイルから読む。**無ければ空。ファイルは作らない。**
    pub fn load_or_empty(path: &Path) -> Result<Self, ChecksError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(ChecksError::Io {
                detail: error.to_string(),
            }),
        }
    }

    pub fn get(&self, id: &str) -> Option<&Check> {
        self.checks.iter().find(|check| check.id == id)
    }

    /// 全部。**人が書いた並びのまま。**
    pub fn checks(&self) -> &[Check] {
        &self.checks
    }

    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    fn validate(&self) -> Result<(), ChecksError> {
        if self.version != CHECKS_VERSION {
            return Err(ChecksError::UnknownVersion {
                found: self.version,
            });
        }
        let mut seen = HashSet::new();
        for check in &self.checks {
            if check.id.trim().is_empty() {
                return Err(ChecksError::EmptyId);
            }
            if !seen.insert(check.id.as_str()) {
                return Err(ChecksError::DuplicateId {
                    id: check.id.clone(),
                });
            }
            if check.steps.is_empty() {
                return Err(ChecksError::NoSteps {
                    id: check.id.clone(),
                });
            }
            if check
                .steps
                .iter()
                .any(|step| step.readonly.trim().is_empty())
            {
                return Err(ChecksError::EmptyStep {
                    id: check.id.clone(),
                });
            }
        }
        Ok(())
    }
}
