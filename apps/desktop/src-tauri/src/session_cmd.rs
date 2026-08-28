//! 画面からサーバーを触る口。**MCP と同じ [`Engine`] を通ります**（PRD §4-1）。
//!
//! ここは**人の操作**なので、書き込みの囲い（D22）はかかりません。
//! 囲いは AI の口にだけかかります（PRD §3「人（GUI）の側は制限しない」）。

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use sshboard_band::Actor;
use sshboard_engine::{Engine, Opened};
use tauri::{AppHandle, Emitter, State};

/// 開いている接続が変わったことを画面へ配るイベント。
pub const SESSION_EVENT: &str = "session://changed";

/// 画面へ返すディレクトリの 1 件。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// 開いている接続の変化を画面へ押し出す。
///
/// **AI が繋いだり切ったりしたときに、人が知らないままにならないため**（PRD §4-2）。
pub fn spawn_bridge(app: AppHandle, engine: Arc<Engine>) {
    tauri::async_runtime::spawn(async move {
        let mut watching = engine.subscribe();
        while watching.changed().await.is_ok() {
            let current: Option<Opened> = watching.borrow().clone();
            if let Err(error) = app.emit(SESSION_EVENT, current) {
                eprintln!("[sshboard] 接続の変化を画面へ渡せません: {error}");
            }
        }
    });
}

/// 繋ぐ。`passphrase` は**人がその場で入れたもの**だけ（D14）。
#[tauri::command]
pub async fn session_connect(
    id: String,
    passphrase: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Opened, String> {
    // 空文字は「入れていない」。空のまま鍵へ渡すと、読めない理由が分かりにくくなる。
    let passphrase = passphrase.filter(|value| !value.is_empty());
    engine
        .connect(Actor::Human, &id, passphrase)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn session_disconnect(engine: State<'_, Arc<Engine>>) -> Result<Option<Opened>, String> {
    Ok(engine.disconnect(Actor::Human).await)
}

#[tauri::command]
pub async fn session_status(engine: State<'_, Arc<Engine>>) -> Result<Option<Opened>, String> {
    Ok(engine.current().await)
}

#[tauri::command]
pub async fn remote_list_dir(
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Listed>, String> {
    let entries = engine
        .list_dir(Actor::Human, &path)
        .await
        .map_err(|error| error.to_string())?;

    let mut listed: Vec<Listed> = entries
        .into_iter()
        .map(|entry| Listed {
            name: entry.name,
            is_dir: entry.is_dir,
            size: entry.size,
        })
        .collect();

    // ディレクトリを先に、あとは名前順。**並びが毎回変わると人が見失う。**
    listed.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(listed)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResult {
    pub bytes: u64,
    /// UTF-8 として読めないバイトがあったか。**画面に出す。**
    pub was_lossy: bool,
    pub text: String,
}

/// ファイルを読む。**文字コードはここで決めます**（画面に出すため）。
///
/// UTF-8 でない中身が実在するので（EUC-JP のログ・Issue 002）、
/// **読めなかったことを黙らず、画面へ伝えます。**
#[tauri::command]
pub async fn remote_read_file(
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<ReadResult, String> {
    let bytes = engine
        .read_file(Actor::Human, &path)
        .await
        .map_err(|error| error.to_string())?;

    let text = String::from_utf8_lossy(&bytes);
    Ok(ReadResult {
        bytes: bytes.len() as u64,
        was_lossy: matches!(text, std::borrow::Cow::Owned(_)),
        text: text.into_owned(),
    })
}

#[tauri::command]
pub async fn remote_ensure_dir(path: String, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine
        .ensure_dir(Actor::Human, &path)
        .await
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Uploaded {
    pub name: String,
    pub remote: String,
    pub bytes: u64,
}

/// 手元のファイルを上げる。**1 件ずつ帯に出ます。**
#[tauri::command]
pub async fn remote_upload(
    local_paths: Vec<String>,
    remote_dir: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Uploaded>, String> {
    let mut done = Vec::with_capacity(local_paths.len());
    for local in local_paths {
        let path = PathBuf::from(&local);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("ファイル名を取り出せません: {local}"))?;
        let remote = join_remote(&remote_dir, &name);

        // **1 件失敗したら、そこで止める。**残りを黙って続けると、
        // どこまで上がったのかが分からなくなる。
        let bytes = engine
            .upload_file(Actor::Human, &path, &remote)
            .await
            .map_err(|error| format!("{name}: {error}"))?;
        done.push(Uploaded {
            name,
            remote,
            bytes,
        });
    }
    Ok(done)
}

/// リモートのディレクトリとファイル名を繋ぐ。**`//` を作らない。**
fn join_remote(dir: &str, name: &str) -> String {
    let dir = dir.trim_end_matches('/');
    format!("{dir}/{name}")
}

#[cfg(test)]
mod tests {
    use super::join_remote;

    #[test]
    fn joining_a_directory_and_a_name_never_doubles_the_separator() {
        assert_eq!(join_remote("/srv/app", "a.tar.gz"), "/srv/app/a.tar.gz");
        assert_eq!(join_remote("/srv/app/", "a.tar.gz"), "/srv/app/a.tar.gz");
        assert_eq!(join_remote("/", "a.tar.gz"), "/a.tar.gz");
    }
}
