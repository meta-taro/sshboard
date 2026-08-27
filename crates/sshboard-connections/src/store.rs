//! `connections.toml` の読み書き。
//!
//! **無いことは異常ではありません。**まだ 1 件も登録していないだけです。
//! 読むときにファイルを作りません。**保存して初めて出来ます。**
//!
//! Unix では `0o600` で書きます。**同じ機械の他の利用者に読ませない。**

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::entry::{ConnectionEntry, ConnectionSummary};
use crate::mark::{is_connection_color, is_connection_tag, CONNECTION_TAG_MAX_CHARS};

/// いまの書式。**上げたら、読めない版を黙って捨てない。**
pub const CURRENT_VERSION: u32 = 1;

/// 読めなかった理由。**握り潰さない。**
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionsError {
    /// 書式が読めない。
    Malformed { detail: String },
    /// 知らない版。**捨てずに止める。**
    UnknownVersion { found: u32 },
    /// 同じ識別子が 2 つ。**どちらを使うか決められない。**
    DuplicateId { id: String },
    /// 識別子が空。
    EmptyId,
    /// 配色に無い色。**書くと、対応する定義が無い値がファイルに入る。**
    UnknownColor { id: String, color: String },
    /// タグが行に載らない長さ。
    TagTooLong { id: String, chars: usize },
    /// ファイルを触れない。
    Io { detail: String },
    /// 置き場所が分からない。
    NoConfigDirectory,
}

impl std::fmt::Display for ConnectionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionsError::Malformed { detail } => write!(f, "接続の一覧を読めません: {detail}"),
            ConnectionsError::UnknownVersion { found } => {
                write!(f, "知らない版の接続一覧です（version = {found}）")
            }
            ConnectionsError::DuplicateId { id } => write!(f, "識別子が重複しています: {id}"),
            ConnectionsError::EmptyId => write!(f, "識別子が空の接続があります"),
            ConnectionsError::UnknownColor { id, color } => {
                write!(f, "`{id}` の色 `{color}` は配色にありません")
            }
            ConnectionsError::TagTooLong { id, chars } => {
                write!(f, "`{id}` のタグが長すぎます（{chars} 文字・上限 {CONNECTION_TAG_MAX_CHARS} 文字）")
            }
            ConnectionsError::Io { detail } => write!(f, "接続の一覧を触れません: {detail}"),
            ConnectionsError::NoConfigDirectory => write!(f, "設定の置き場所が分かりません"),
        }
    }
}

impl std::error::Error for ConnectionsError {}

/// 登録された接続の一覧。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connections {
    pub version: u32,
    #[serde(default)]
    pub connections: Vec<ConnectionEntry>,
}

impl Connections {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            connections: Vec::new(),
        }
    }

    /// TOML から読む。**版と重複をここで弾く。**
    pub fn parse(input: &str) -> Result<Self, ConnectionsError> {
        let parsed: Self = toml::from_str(input).map_err(|error| ConnectionsError::Malformed {
            detail: error.to_string(),
        })?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn to_toml(&self) -> Result<String, ConnectionsError> {
        toml::to_string_pretty(self).map_err(|error| ConnectionsError::Malformed {
            detail: error.to_string(),
        })
    }

    /// ファイルから読む。**無ければ空を返す。ファイルは作らない。**
    pub fn load_or_empty(path: &Path) -> Result<Self, ConnectionsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            // まだ 1 件も登録していないだけ。**異常ではない。**
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(ConnectionsError::Io {
                detail: error.to_string(),
            }),
        }
    }

    /// ファイルへ書く。Unix では `0o600`。
    pub fn save(&self, path: &Path) -> Result<(), ConnectionsError> {
        // 壊れた一覧を書き出さない。**読めないファイルを残す方が困る。**
        self.validate()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ConnectionsError::Io {
                detail: error.to_string(),
            })?;
        }

        write_private(path, &self.to_toml()?)
    }

    /// **版・空の識別子・重複をここで弾く。**黙って落とすと、
    /// 人は「登録したはずなのに無い」に遭う。
    fn validate(&self) -> Result<(), ConnectionsError> {
        if self.version != CURRENT_VERSION {
            return Err(ConnectionsError::UnknownVersion {
                found: self.version,
            });
        }

        let mut seen = std::collections::HashSet::new();
        for entry in &self.connections {
            if entry.id.trim().is_empty() {
                return Err(ConnectionsError::EmptyId);
            }
            if !seen.insert(entry.id.as_str()) {
                return Err(ConnectionsError::DuplicateId {
                    id: entry.id.clone(),
                });
            }

            // **配色に無い色を書かせない。**書くと、対応する定義が無い値がファイルに入る。
            if let Some(color) = &entry.color {
                if !is_connection_color(color) {
                    return Err(ConnectionsError::UnknownColor {
                        id: entry.id.clone(),
                        color: color.clone(),
                    });
                }
            }

            // **文字数で測る。**バイトで測ると、漢字の短いラベルを弾いてしまう。
            if let Some(tag) = &entry.tag {
                if !is_connection_tag(tag) {
                    return Err(ConnectionsError::TagTooLong {
                        id: entry.id.clone(),
                        chars: tag.chars().count(),
                    });
                }
            }
        }
        Ok(())
    }

    /// **AI へ渡す形。**識別子と名前だけ。
    pub fn summaries(&self) -> Vec<ConnectionSummary> {
        self.connections
            .iter()
            .map(ConnectionEntry::summary)
            .collect()
    }

    /// 手元で使うための取り出し。**AI へは渡さない。**
    pub fn get(&self, id: &str) -> Option<&ConnectionEntry> {
        self.connections.iter().find(|entry| entry.id == id)
    }
}

/// 既定の置き場所。
///
/// - Windows: `%APPDATA%\sshboard\sshboard\config\connections.toml`
/// - macOS: `~/Library/Application Support/dev.sshboard.sshboard/connections.toml`
pub fn default_path() -> Result<PathBuf, ConnectionsError> {
    directories::ProjectDirs::from("dev", "sshboard", "sshboard")
        .map(|dirs| dirs.config_dir().join("connections.toml"))
        .ok_or(ConnectionsError::NoConfigDirectory)
}

/// **同じ機械の他の利用者に読ませない。**
///
/// `mode()` は新規作成時にしか効かないので、既にあるファイルは書いたあとに締め直す。
#[cfg(unix)]
fn write_private(path: &Path, text: &str) -> Result<(), ConnectionsError> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let io = |error: std::io::Error| ConnectionsError::Io {
        detail: error.to_string(),
    };

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(io)?;
    file.write_all(text.as_bytes()).map_err(io)?;
    file.flush().map_err(io)?;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(io)
}

/// Windows は継承した ACL に任せる（`%APPDATA%` は利用者ごと）。
#[cfg(not(unix))]
fn write_private(path: &Path, text: &str) -> Result<(), ConnectionsError> {
    std::fs::write(path, text).map_err(|error| ConnectionsError::Io {
        detail: error.to_string(),
    })
}
