//! 接続情報の書き出し／取り込み（D18）。**人が使う口です。**
//!
//! **AI へは渡しません。**MCP 側にこの口はありません。
//! バンドルは接続先と鍵のパスフレーズを丸ごと含むので、
//! 「AI の書き込みを囲いの外へ出さない」（D22）以前の話として、
//! **AI が触ってよいものではありません。**
//!
//! 段取りと暗号は `sshboard-bundle` が持っています。**ここは薄い橋渡し**です。

use std::path::PathBuf;

use sshboard_band::{Actor, Band};
use sshboard_bundle::{
    apply_payload, build_payload, decrypt_bundle, encrypt_bundle, validate_passphrase, SecretVault,
};
use sshboard_connections::Connections;
use sshboard_credentials::SecretStore;
use tauri::State;

use crate::connections_cmd::ConnectionsPath;

/// OS の資格情報ストアでの区分名。**`sshboard-engine` と揃えること。**
const KEYRING_SERVICE: &str = "sshboard";

/// 製品の保管庫。試験では別の実装に差し替わります。
struct OsVault(SecretStore);

impl OsVault {
    fn new() -> Self {
        Self(SecretStore::new(KEYRING_SERVICE))
    }
}

impl SecretVault for OsVault {
    fn get(&self, reference: &str) -> Result<String, String> {
        self.0.get(reference).map_err(|error| error.to_string())
    }
    fn put(&self, reference: &str, secret: &str) -> Result<(), String> {
        self.0.put(reference, secret).map_err(|e| e.to_string())
    }
    fn delete(&self, reference: &str) -> Result<(), String> {
        self.0.delete(reference).map_err(|e| e.to_string())
    }
}

fn load(path: &ConnectionsPath) -> Result<Connections, String> {
    Connections::load_or_empty(&path.0).map_err(|error| error.to_string())
}

/// 選んだ接続を、パスフレーズで暗号化して 1 ファイルへ書き出す。
///
/// **接続先を帯へ出しません。**出すのは件数だけ（PRD §8）。
///
/// # Errors
///
/// パスフレーズが短い、識別子が無い、秘密を取り出せない、書けない場合。
#[tauri::command]
pub fn bundle_export(
    ids: Vec<String>,
    passphrase: String,
    destination: String,
    path: State<'_, ConnectionsPath>,
    band: State<'_, Band>,
) -> Result<usize, String> {
    // **暗号化を始める前に断る。**長い処理のあとで「短すぎます」は不親切。
    validate_passphrase(&passphrase).map_err(|error| error.to_string())?;
    if ids.is_empty() {
        return Err("書き出す接続が選ばれていません".into());
    }

    let all = load(&path)?;
    let vault = OsVault::new();
    let payload = build_payload(&all, &ids, &vault).map_err(|error| error.to_string())?;
    let blob = encrypt_bundle(&payload, &passphrase).map_err(|error| error.to_string())?;

    let destination = PathBuf::from(destination);
    std::fs::write(&destination, &blob).map_err(|error| format!("書き出せません: {error}"))?;

    // **件数だけ。**接続先も、書き出し先のパスも帯へ載せません。
    band.record(
        Actor::Human,
        format!("接続を {} 件、暗号化して書き出しました", ids.len()),
    );
    Ok(ids.len())
}

/// 書き出したファイルを取り込む。
///
/// **秘密を保管庫へ入れてから一覧を保存します。**順番が逆だと、
/// 一覧には載っているのに秘密が無い接続ができます。
///
/// # Errors
///
/// 読めない、パスフレーズが違う、保管庫へ入れられない、保存できない場合。
#[tauri::command]
pub fn bundle_import(
    source: String,
    passphrase: String,
    path: State<'_, ConnectionsPath>,
    band: State<'_, Band>,
) -> Result<usize, String> {
    let blob = std::fs::read(&source).map_err(|error| format!("読めません: {error}"))?;
    let payload = decrypt_bundle(&blob, &passphrase).map_err(|error| error.to_string())?;
    let count = payload.connections.connections.len();

    let current = load(&path)?;
    let vault = OsVault::new();
    let merged = apply_payload(payload, current, &vault).map_err(|error| error.to_string())?;

    merged
        .save(&path.0)
        .map_err(|error| format!("保存できません: {error}"))?;

    band.record(Actor::Human, format!("接続を {count} 件、取り込みました"));
    Ok(count)
}
