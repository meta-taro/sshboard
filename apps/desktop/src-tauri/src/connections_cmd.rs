//! 接続の登録・編集・削除。**人が使う口です。**
//!
//! **ここは AI へ渡す口ではありません。**画面（人）はホスト名も利用者名も見ます。
//! AI が見るのは `list_connections`（MCP 側）が返す識別子と名前だけです
//! （CLAUDE.md 禁止事項 5）。
//!
//! 中身の検証・保存・権限は `sshboard-connections` が持っています。
//! **ここは薄い橋渡しだけ**にして、試験はそちらに置きます。

use std::path::PathBuf;
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_connections::{ConnectionEntry, Connections, ConnectionsWatch};
use sshboard_credentials::SecretStore;
use tauri::State;

/// 接続一覧の置き場所。起動時に決めて持っておく。
pub struct ConnectionsPath(pub PathBuf);

/// OS の資格情報ストアでの区分名。**`sshboard-engine` と揃えること。**
const KEYRING_SERVICE: &str = "sshboard";

fn load(path: &ConnectionsPath) -> Result<Connections, String> {
    Connections::load_or_empty(&path.0).map_err(|error| error.to_string())
}

/// 人が触ったことも帯へ出す（PRD §4-2「面が違っても記録は 1 本」）。
///
/// **識別子だけを載せる。**ホスト名を帯へ出すと、画面の写真に接続先が写る（PRD §8）。
fn record(band: &Band, text: String) {
    band.record(Actor::Human, text);
}

#[tauri::command]
pub fn connections_list(path: State<'_, ConnectionsPath>) -> Result<Vec<ConnectionEntry>, String> {
    Ok(load(&path)?.connections)
}

/// 追加と更新の両方。同じ識別子があれば置き換える。
#[tauri::command]
pub fn connection_save(
    entry: ConnectionEntry,
    path: State<'_, ConnectionsPath>,
    band: State<'_, Band>,
    watch: State<'_, Arc<ConnectionsWatch>>,
) -> Result<(), String> {
    let mut connections = load(&path)?;

    let existed = connections
        .connections
        .iter()
        .any(|held| held.id == entry.id);
    let id = entry.id.clone();

    // **作り直して差し替える。**元の Vec を書き換えない。
    let next: Vec<ConnectionEntry> = if existed {
        connections
            .connections
            .into_iter()
            .map(|held| {
                if held.id == entry.id {
                    entry.clone()
                } else {
                    held
                }
            })
            .collect()
    } else {
        connections
            .connections
            .into_iter()
            .chain(std::iter::once(entry))
            .collect()
    };
    connections = Connections {
        version: connections.version,
        connections: next,
    };

    connections
        .save(&path.0)
        .map_err(|error| error.to_string())?;

    record(
        &band,
        format!(
            "{} 接続: {id}",
            if existed {
                "更新した"
            } else {
                "登録した"
            }
        ),
    );
    watch.notify();
    Ok(())
}

#[tauri::command]
pub fn connection_delete(
    id: String,
    path: State<'_, ConnectionsPath>,
    band: State<'_, Band>,
    watch: State<'_, Arc<ConnectionsWatch>>,
) -> Result<(), String> {
    let connections = load(&path)?;

    let next: Vec<ConnectionEntry> = connections
        .connections
        .into_iter()
        .filter(|held| held.id != id)
        .collect();

    Connections {
        version: connections.version,
        connections: next,
    }
    .save(&path.0)
    .map_err(|error| error.to_string())?;

    record(&band, format!("消した接続: {id}"));
    watch.notify();
    Ok(())
}

/// 一覧の置き場所を画面へ見せる。**人が直接開けるように。**
#[tauri::command]
pub fn connections_path(path: State<'_, ConnectionsPath>) -> String {
    path.0.display().to_string()
}

/// 鍵ファイルについて画面へ返すこと（D28）。
///
/// **中身もパスも返しません。**返すのは「読めたか・使えるか・パスフレーズが要るか」と、
/// 人に見せる形式の名前だけです。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyReport {
    /// ファイルとして読めたか。**無いのか、形式が違うのかを分ける。**
    pub readable: bool,
    /// 認証に使えるか。公開鍵と、鍵でないものは使えない。
    pub usable: bool,
    /// パスフレーズが要るか。
    pub needs_passphrase: bool,
    /// **鍵ではあるが、この暗号方式を読めない。**
    ///
    /// 「秘密鍵を指してください」と言うと的外れになります（指しているので）。
    /// 別の文言を出すために、使えない理由をここで分けます。
    pub unsupported_encryption: bool,
    /// 形式の名前（`OpenSSH` / `PuTTY (PPK v3)` など）。
    pub format: String,
}

/// 指した鍵が何なのかを、**中身だけで**判定して返す（D28）。
///
/// **拡張子を見ません。**`*.tera.ppk` の中身が OpenSSH 秘密鍵だった、が実際に在り、
/// 拡張子で判定していた頃は**要らない変換作業へ人を送っていました。**
#[tauri::command]
pub fn inspect_key_file(path: String) -> KeyReport {
    let Ok(bytes) = std::fs::read(&path) else {
        return KeyReport {
            readable: false,
            usable: false,
            needs_passphrase: false,
            unsupported_encryption: false,
            format: String::new(),
        };
    };

    let facts = sshboard_engine::inspect_key(&bytes);
    KeyReport {
        readable: true,
        usable: facts.usable(),
        needs_passphrase: facts.needs_passphrase,
        unsupported_encryption: facts.verdict == sshboard_engine::KeyVerdict::UnsupportedEncryption,
        format: facts.format.label().to_owned(),
    }
}

/// 一覧が変わったことを画面へ流し続ける。
///
/// **中身は流さない。**「変わった」だけを送り、画面が読み直す。
/// 中身を流すと、イベントに接続先が乗る（PRD §8）。
pub fn spawn_bridge(app: tauri::AppHandle, watch: Arc<ConnectionsWatch>) {
    use tauri::Emitter;

    let mut changes = watch.subscribe();
    tauri::async_runtime::spawn(async move {
        while changes.recv().await.is_ok() {
            if let Err(error) = app.emit(CONNECTIONS_CHANGED_EVENT, ()) {
                eprintln!("[sshboard] 接続一覧の変更を画面へ渡せません: {error}");
            }
        }
    });
}

/// 画面が待ち受けるイベント名。
pub const CONNECTIONS_CHANGED_EVENT: &str = "connections://changed";

/// ログインのパスワードを OS の資格情報ストアへ預ける。
///
/// **接続に入るのは参照名だけ**で、パスワードそのものは入りません（D11）。
/// **投入するのは人だけ**です — MCP にこの口はありません（§14）。
///
/// 空文字を渡すと**預けたものを消します**。「もう使わない」を言う手段が要るためです。
#[tauri::command]
pub fn connection_password_save(
    id: String,
    password: String,
    path: State<'_, ConnectionsPath>,
    band: State<'_, Band>,
    watch: State<'_, Arc<ConnectionsWatch>>,
) -> Result<(), String> {
    let store = SecretStore::new(KEYRING_SERVICE);
    let reference = format!("password:{id}");

    let mut connections = load(&path)?;
    let found = connections
        .connections
        .iter()
        .any(|held| held.id == id)
        .then_some(())
        .ok_or_else(|| format!("そんな接続はありません: {id}"))?;
    let _ = found;

    if password.is_empty() {
        // 消せなくても進みます。**参照を外す方が大事**です
        // （残すと「あるはず」で繋ぎに行って失敗します）。
        let _ = store.delete(&reference);
    } else {
        store
            .put(&reference, &password)
            .map_err(|error| format!("OS の資格情報ストアへ預けられません: {error}"))?;
    }

    let next: Vec<ConnectionEntry> = connections
        .connections
        .into_iter()
        .map(|mut held| {
            if held.id == id {
                held.keyring_password_ref =
                    (!password.is_empty()).then(|| reference.clone());
            }
            held
        })
        .collect();
    connections.connections = next;
    connections
        .save(&path.0)
        .map_err(|error| error.to_string())?;

    // **識別子だけを載せる。**パスワードも接続先も帯へ出しません（PRD §8）。
    record(
        &band,
        if password.is_empty() {
            format!("接続 `{id}` のパスワードを消しました")
        } else {
            format!("接続 `{id}` のパスワードを預かりました")
        },
    );
    watch.notify();
    Ok(())
}

/// パスワードを預けてあるか。**中身は返しません**（あるか無いかだけ）。
#[tauri::command]
pub fn connection_has_password(id: String, path: State<'_, ConnectionsPath>) -> Result<bool, String> {
    Ok(load(&path)?
        .connections
        .iter()
        .any(|held| held.id == id && held.keyring_password_ref.is_some()))
}
