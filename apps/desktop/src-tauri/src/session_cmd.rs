//! 画面からサーバーを触る口。**MCP と同じ [`Engine`] を通ります**（PRD §4-1）。
//!
//! ここは**人の操作**なので、書き込みの囲い（D22）はかかりません。
//! 囲いは AI の口にだけかかります（PRD §3「人（GUI）の側は制限しない」）。

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use sshboard_band::Actor;
use sshboard_diag::Event;
use sshboard_engine::{Engine, OnConflict, Opened};
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
            // **開いているもの全部**を流す（D25）。タブに 1 本残らず出すため。
            let current: Vec<Opened> = watching.borrow().clone();
            if let Err(error) = app.emit(SESSION_EVENT, current) {
                eprintln!("[sshboard] 接続の変化を画面へ渡せません: {error}");
            }
        }
    });
}

/// 繋げなかった理由。**文字列に潰さない。**
///
/// 画面が「この指紋で登録しますか」「パスフレーズを入れてください」を出せないと、
/// 人はそこで行き止まりになる（**実際になった**）。
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConnectFailure {
    /// 初見のホスト。**人が確かめて登録すれば繋がる。**
    Untrusted {
        algorithm: String,
        fingerprint: String,
        /// 登録済みの指紋。**あるのに食い違うなら、すり替えの疑い。**
        expected: Option<String>,
    },
    /// 鍵にパスフレーズが要る。
    PassphraseNeeded,
    /// それ以外。**そのまま人へ見せる。**
    Other { message: String },
}

impl From<sshboard_engine::EngineError> for ConnectFailure {
    fn from(error: sshboard_engine::EngineError) -> Self {
        use sshboard_engine::EngineError;
        match error {
            EngineError::UntrustedHost {
                algorithm,
                fingerprint,
                expected,
                ..
            } => ConnectFailure::Untrusted {
                algorithm,
                fingerprint,
                expected,
            },
            EngineError::PassphraseNeeded { .. } => ConnectFailure::PassphraseNeeded,
            other => ConnectFailure::Other {
                message: other.to_string(),
            },
        }
    }
}

/// 繋ぐ。`passphrase` は**人がその場で入れたもの**だけ（D14）。
#[tauri::command]
pub async fn session_connect(
    id: String,
    passphrase: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Opened, ConnectFailure> {
    // 空文字は「入れていない」。空のまま鍵へ渡すと、読めない理由が分かりにくくなる。
    let passphrase = passphrase.filter(|value| !value.is_empty());
    engine
        .connect(Actor::Human, &id, passphrase)
        .await
        .map_err(ConnectFailure::from)
}

/// 切る。`id` を省略すると、いまの宛先。
#[tauri::command]
pub async fn session_disconnect(
    id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<Opened>, String> {
    Ok(engine.disconnect(Actor::Human, id.as_deref()).await)
}

/// 開いているもの全部と、いまの宛先。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    /// **1 本残らず**（D25）。タブに出すのはこれ。
    pub open: Vec<Opened>,
    /// 操作がどれへ行くか。
    pub active: Option<String>,
}

#[tauri::command]
pub async fn session_status(engine: State<'_, Arc<Engine>>) -> Result<SessionStatus, String> {
    Ok(SessionStatus {
        open: engine.open_connections().await,
        active: engine.active().await.map(|open| open.id),
    })
}

/// 操作の宛先を変える。**タブを押したとき。**
#[tauri::command]
pub async fn session_focus(id: String, engine: State<'_, Arc<Engine>>) -> Result<Opened, String> {
    engine.focus(&id).await.map_err(|error| error.to_string())
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloaded {
    pub name: String,
    pub local: String,
    pub bytes: u64,
}

/// サーバーのファイルを手元へ落とす。**1 件ずつ帯に出ます。**
///
/// `overwrite` が真になるのは、**人が画面で「上書きする」を選んだとき**だけです。
/// 既定では、同じ名前が手元にあれば断ります（product-baseline §13）。
#[tauri::command]
pub async fn remote_download(
    names: Vec<String>,
    remote_dir: String,
    local_dir: String,
    overwrite: bool,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Downloaded>, String> {
    let local_dir = PathBuf::from(&local_dir);
    if !local_dir.is_dir() {
        return Err(format!(
            "{} というディレクトリがありません",
            local_dir.display()
        ));
    }
    let on_conflict = if overwrite {
        OnConflict::Overwrite
    } else {
        OnConflict::Refuse
    };

    let mut done = Vec::with_capacity(names.len());
    for requested in &names {
        let name = safe_local_name(requested)?;
        let local = local_dir.join(name);

        // **名前の検査だけに頼らない。**Windows の `C:x`（ドライブ相対）のように、
        // 区切り文字が無いのに階層が変わる書き方がある。
        // 実際に繋いだ結果が、人が選んだディレクトリの直下かどうかで見る。
        if local.parent() != Some(local_dir.as_path()) {
            return Err(format!("手元へ落とせない名前です: {name}"));
        }

        // **1 件失敗したら、そこで止める**（上げる側と同じ）。
        // 残りを黙って続けると、どこまで落ちたのかが分からなくなる。
        let bytes = engine
            .download_file(
                Actor::Human,
                &join_remote(&remote_dir, name),
                &local,
                on_conflict,
            )
            .await
            .map_err(|error| format!("{name}: {error}"))?;
        done.push(Downloaded {
            name: name.to_owned(),
            local: local.to_string_lossy().into_owned(),
            bytes,
        });
    }
    Ok(done)
}

/// サーバーから来た名前を、**落とし先の 1 階層分の名前として**受け取る。
///
/// **一覧を返すのはサーバー**です。`../` を含む名前を返されたまま繋ぐと、
/// **繋いだ相手が手元の好きな場所へ書ける**ことになります。
/// ここは「1 語であること」だけを通します（許可リストの考え方・D3 と同じ）。
fn safe_local_name(name: &str) -> Result<&str, String> {
    let ok = !name.trim().is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0');

    if ok {
        Ok(name)
    } else {
        Err(format!("手元へ落とせない名前です: {name}"))
    }
}

/// 手元のディレクトリ 1 つ分。
///
/// **左右で同じ形にする。**片方だけ違う形だと、画面で並べたときに揃わない。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalListing {
    pub path: String,
    /// 1 つ上。**根まで来たら `None`**（上へ行き過ぎない）。
    pub parent: Option<String>,
    pub entries: Vec<Listed>,
}

/// 手元のディレクトリを読む。**中身は読みません**（名前・種別・大きさだけ）。
///
/// `path` を省略すると持ち主の home から始めます。
#[tauri::command]
pub async fn local_list_dir(path: Option<String>) -> Result<LocalListing, String> {
    let here = match path.filter(|value| !value.trim().is_empty()) {
        Some(value) => PathBuf::from(value),
        None => home_dir().ok_or("手元の起点が分かりません")?,
    };

    let mut entries = Vec::new();
    let reading = std::fs::read_dir(&here).map_err(|error| {
        // **握り潰さない。**読めない理由（権限・存在しない）が分からないと動けない。
        format!("{} を読めません: {error}", here.display())
    })?;

    for entry in reading.flatten() {
        let Ok(meta) = entry.metadata() else {
            // 1 件読めなくても一覧全体を諦めない。**壊れた symlink で全部消えない。**
            continue;
        };
        entries.push(Listed {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }

    // 左右で同じ並び。**ディレクトリが先、あとは名前順。**
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(LocalListing {
        parent: here.parent().map(|up| up.to_string_lossy().into_owned()),
        path: here.to_string_lossy().into_owned(),
        entries,
    })
}

/// 持ち主の home。**Windows も見る**（配布対象・PRD §7）。
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// リモートのディレクトリとファイル名を繋ぐ。**`//` を作らない。**
fn join_remote(dir: &str, name: &str) -> String {
    let dir = dir.trim_end_matches('/');
    format!("{dir}/{name}")
}

#[cfg(test)]
mod tests {
    use super::{join_remote, safe_local_name};

    #[test]
    fn joining_a_directory_and_a_name_never_doubles_the_separator() {
        assert_eq!(join_remote("/srv/app", "a.tar.gz"), "/srv/app/a.tar.gz");
        assert_eq!(join_remote("/srv/app/", "a.tar.gz"), "/srv/app/a.tar.gz");
        assert_eq!(join_remote("/", "a.tar.gz"), "/a.tar.gz");
    }

    #[test]
    fn an_ordinary_file_name_is_accepted() {
        assert_eq!(safe_local_name("app.tar.gz"), Ok("app.tar.gz"));
        assert_eq!(
            safe_local_name("日本語 の 名前.txt"),
            Ok("日本語 の 名前.txt")
        );
    }

    #[test]
    fn a_name_that_climbs_out_of_the_chosen_directory_is_refused() {
        // **落とすファイル名を決めるのはサーバー**です（一覧はサーバーが返す）。
        // ここを通すと、繋いだ相手が手元の任意の場所へ書けることになります。
        for hostile in [
            "../secret",
            "../../.ssh/authorized_keys",
            "sub/dir.txt",
            "sub\\dir.txt",
            "/etc/passwd",
            "C:\\Windows\\x.dll",
            "..",
            ".",
            "",
            "   ",
        ] {
            assert!(
                safe_local_name(hostile).is_err(),
                "{hostile} を落とし先の名前として通している"
            );
        }
    }
}

/// 何が起きたかの記録。**新しい順。**
///
/// 画面と MCP で**同じ 1 つ**を見ます。片方にしか出ない失敗を作らないため。
#[tauri::command]
pub async fn diagnostics_recent(
    limit: Option<usize>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Event>, String> {
    Ok(engine.diagnostics().recent(limit.unwrap_or(200)))
}

// --- 端末（D29） -------------------------------------------------------------

/// 握りが変わったことを画面へ配るイベント。
pub const CONSOLE_EVENT: &str = "console://holder";

/// 誰が握っているか。**画面はこれを見て入力を締めます。**
fn holder_name(holder: Option<Actor>) -> Option<&'static str> {
    holder.map(|held| match held {
        Actor::Human => "human",
        Actor::Ai => "ai",
    })
}

/// 握りの変化を画面へ押し出す。
///
/// **AI が握った瞬間に、人の側の入力が締まる**必要があります（D29）。
/// 画面が知らないまま AI が打っている、を作らないため。
pub fn spawn_console_bridge(app: AppHandle, engine: Arc<Engine>) {
    tauri::async_runtime::spawn(async move {
        let mut watching = engine.subscribe_console();
        while watching.changed().await.is_ok() {
            let holder = holder_name(*watching.borrow());
            if let Err(error) = app.emit(CONSOLE_EVENT, holder) {
                eprintln!("[sshboard] 握りの変化を画面へ渡せません: {error}");
            }
        }
    });
}

/// 端末を開いて握る。**人の操作。**
#[tauri::command]
pub async fn console_open(
    cols: u32,
    rows: u32,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    engine
        .console_open(Actor::Human, cols.max(20), rows.max(5))
        .await
        .map_err(|error| error.to_string())
}

/// 打ち込む。**握っている側だけ**（Engine が判断します）。
///
/// **帯へは載せません**（D29）。1 キーずつ載せると帯が溢れ、
/// 受け取り待ちを挟むと端末が使い物になりません。
#[tauri::command]
pub async fn console_type(bytes: Vec<u8>, engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine
        .console_type(Actor::Human, &bytes)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn console_resize(
    cols: u32,
    rows: u32,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    engine
        .console_resize(cols.max(20), rows.max(5))
        .await
        .map_err(|error| error.to_string())
}

/// 取り返す。**人は常に勝ちます**（D29）。
#[tauri::command]
pub async fn console_take(engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine
        .console_take(Actor::Human)
        .await
        .map_err(|error| error.to_string())
}

/// 止める。**失敗しません**（D29 の停止ボタン）。
#[tauri::command]
pub async fn console_stop(engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine.console_stop().await;
    Ok(())
}

/// いま誰が握っているか。**画面を開いた直後に読みます。**
#[tauri::command]
pub async fn console_holder(engine: State<'_, Arc<Engine>>) -> Result<Option<String>, String> {
    Ok(holder_name(engine.console_holder().await).map(|name| name.to_string()))
}
