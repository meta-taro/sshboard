//! 開いている 1 本を持ち、すべての操作をここへ集める。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_connections::{ConnectionEntry, Connections};
use sshboard_credentials::SecretStore;
use sshboard_diag::{Diagnostics, Stage};
use sshboard_ssh::{Auth, DirEntry, SshSession, Target, WriteScope};
use sshboard_stream::OutputStream;
use tokio::sync::{watch, Mutex};

use crate::error::EngineError;
use crate::open::{Opened, WriteAccess};

/// OS ストアの区分名。**接続一覧には参照名しか置かない**（D11）。
const KEYRING_SERVICE: &str = "sshboard";

struct Live {
    session: Arc<SshSession>,
    opened: Opened,
}

/// 開いているもの全部と、いま操作の宛先になっているもの。
///
/// **1 本残らずここに入ります**（D25）。裏に持つ場所はありません。
#[derive(Default)]
struct Held {
    /// 識別子で引く。**並びが毎回変わるとタブが踊る**ので BTreeMap。
    live: BTreeMap<String, Live>,
    /// いまの宛先。**閉じたら次の 1 本へ移る**（宛先が無いまま開いている、を作らない）。
    active: Option<String>,
}

/// **すべての操作が通る 1 か所**（PRD §4-1）。
pub struct Engine {
    band: Band,
    /// 何が起きたかの記録。**GUI と MCP で同じ 1 つを見る。**
    /// 片方にしか出ない失敗を作らない。
    diag: Diagnostics,
    stream: Arc<OutputStream>,
    connections_path: PathBuf,
    held: Mutex<Held>,
    /// 開いているものが変わったことを配る。**画面が知らないまま繋がっている、を作らない。**
    changed: watch::Sender<Vec<Opened>>,
}

impl Engine {
    pub fn new(band: Band, stream: Arc<OutputStream>, connections_path: PathBuf) -> Self {
        Self::with_diagnostics(band, stream, connections_path, Diagnostics::new())
    }

    pub fn with_diagnostics(
        band: Band,
        stream: Arc<OutputStream>,
        connections_path: PathBuf,
        diag: Diagnostics,
    ) -> Self {
        let (changed, _) = watch::channel(Vec::new());
        Self {
            band,
            diag,
            stream,
            connections_path,
            held: Mutex::new(Held::default()),
            changed,
        }
    }

    /// 開いているものの変化を受け取る口。**全部の一覧が流れます。**
    pub fn subscribe(&self) -> watch::Receiver<Vec<Opened>> {
        self.changed.subscribe()
    }

    /// 開いているもの全部。**タブに出すのはこれ**（D25）。
    pub async fn open_connections(&self) -> Vec<Opened> {
        let held = self.held.lock().await;
        held.live.values().map(|l| l.opened.clone()).collect()
    }

    /// いま操作の宛先になっているもの。
    pub async fn active(&self) -> Option<Opened> {
        let held = self.held.lock().await;
        held.active
            .as_ref()
            .and_then(|id| held.live.get(id))
            .map(|l| l.opened.clone())
    }

    /// 宛先を変える。**開いていないものは指定できない。**
    pub async fn focus(&self, id: &str) -> Result<Opened, EngineError> {
        let mut held = self.held.lock().await;
        let Some(live) = held.live.get(id) else {
            return Err(EngineError::NotConnected);
        };
        let opened = live.opened.clone();
        held.active = Some(id.to_owned());
        let all = held.live.values().map(|l| l.opened.clone()).collect();
        drop(held);
        let _ = self.changed.send(all);
        Ok(opened)
    }

    /// 共有している出力（`tail -f` の行き先）。
    pub fn stream(&self) -> &Arc<OutputStream> {
        &self.stream
    }

    /// 何が起きたかの記録。**人にも AI にも同じものを見せる。**
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diag
    }

    /// 接続一覧の置き場所。
    ///
    /// **MCP 側にも同じ場所を使わせるため。**別々に持つと、
    /// 一覧に見えているものと繋ぎに行くものが食い違いうる。
    pub fn connections_path(&self) -> &Path {
        &self.connections_path
    }

    /// 登録済みの接続へ繋ぐ。
    ///
    /// `passphrase` は **人がその場で入れたものだけ**が入ります。
    /// AI の経路からは常に `None` で呼ばれ、鍵に必要なら
    /// [`EngineError::PassphraseNeeded`] で人へ回します（D14）。
    pub async fn connect(
        &self,
        actor: Actor,
        id: &str,
        passphrase: Option<String>,
    ) -> Result<Opened, EngineError> {
        // **同じ相手を二重に開かない。**別の相手は開ける（D25）。
        {
            let held = self.held.lock().await;
            if let Some(open) = held.live.get(id) {
                return Err(EngineError::AlreadyConnected {
                    id: open.opened.id.clone(),
                    name: open.opened.name.clone(),
                });
            }
        }

        let entry = self.entry(id).inspect_err(|error| {
            self.diag.error(
                Stage::Registry,
                Some(id),
                error.to_string(),
                "接続タブで登録するか、識別子を確かめてください",
            );
        })?;
        let scope = WriteScope::under(&entry.write_roots).map_err(|why| {
            EngineError::Connections(format!("書き込み許可の指定が不正です: {why}"))
        })?;

        let target = Target {
            host: entry.host.clone(),
            port: entry.port,
            user: entry.user.clone(),
            id: Some(entry.id.clone()),
            pinned_fingerprint: entry.fingerprint.clone(),
            known_hosts: read_known_hosts(entry.known_hosts.as_deref()),
            write_scope: scope,
        };
        let auth = self.auth_for(&entry, passphrase)?;

        // **繋ぐ前に帯へ出し、画面が受け取るまで待つ**（D16）。
        // 誰がいつ開いたかが残らないなら、同じ 1 本を共有している意味がない。
        self.show(actor, &format!("connect {}", entry.id)).await?;

        let session = SshSession::connect(&target, &auth, self.band.clone(), &self.diag)
            .await
            // **ホスト鍵の不一致だけは、構造のまま上へ返す。**
            // 文字列に潰すと、画面が「この指紋で登録しますか」を出せず、
            // 人がそこで行き止まりになる（**実際になった**）。
            .map_err(|error| match error {
                sshboard_ssh::SshError::UntrustedHost { seen, trust } => {
                    EngineError::UntrustedHost {
                        id: entry.id.clone(),
                        algorithm: seen.algorithm,
                        fingerprint: seen.fingerprint,
                        expected: match trust {
                            sshboard_ssh::Trust::Mismatch { expected } => Some(expected),
                            _ => None,
                        },
                    }
                }
                other => EngineError::Ssh(other),
            })?;
        let opened = Opened {
            id: entry.id.clone(),
            name: entry.name.clone(),
            tag: entry.tag.clone(),
            fingerprint: session.host_key().fingerprint.clone(),
            host_key_algorithm: session.host_key().algorithm.clone(),
            write: WriteAccess {
                ai_roots: session.write_scope().roots().to_vec(),
                human_unrestricted: true,
            },
        };

        let mut held = self.held.lock().await;
        held.live.insert(
            entry.id.clone(),
            Live {
                session: Arc::new(session),
                opened: opened.clone(),
            },
        );
        // **開いたものを宛先にする。**開いたのに何も向いていない、を作らない。
        held.active = Some(entry.id.clone());
        let all = held.live.values().map(|l| l.opened.clone()).collect();
        drop(held);

        let _ = self.changed.send(all);
        Ok(opened)
    }

    /// 切る。**繋がっていなくても失敗にしない**（同じ状態へ向かう操作なので）。
    ///
    /// `id` を省略すると、いまの宛先を切ります。
    pub async fn disconnect(&self, actor: Actor, id: Option<&str>) -> Option<Opened> {
        let mut held = self.held.lock().await;
        let target = match id {
            Some(id) => id.to_owned(),
            None => held.active.clone()?,
        };
        let closed = held.live.remove(&target).map(|l| l.opened);

        // **宛先が無いまま開いている、を作らない。**残っている 1 本へ移す。
        if held.active.as_deref() == Some(target.as_str()) {
            held.active = held.live.keys().next().cloned();
        }
        let all: Vec<Opened> = held.live.values().map(|l| l.opened.clone()).collect();
        drop(held);

        if let Some(open) = closed.as_ref() {
            // 切断は取り消せないので、**受け取りが返らなくても切る**。
            // ここで失敗にすると「切れないまま繋がっている」という悪い方へ倒れる。
            let _ = self.show(actor, &format!("disconnect {}", open.id)).await;
            self.diag.info(Stage::Reach, Some(&open.id), "切りました");
        }
        let _ = self.changed.send(all);
        closed
    }

    /// 帯へ 1 行載せ、**画面が受け取るまで待つ**（D16）。
    ///
    /// サーバーへ触らない操作（切断）でも通す。**片方だけ見えない、を作らない。**
    async fn show(&self, actor: Actor, text: &str) -> Result<(), EngineError> {
        let delivery = self.band.record(actor, text);
        match delivery.wait_acked(std::time::Duration::from_secs(2)).await {
            sshboard_band::DeliveryOutcome::Delivered => Ok(()),
            sshboard_band::DeliveryOutcome::TimedOut { acked, expected } => Err(EngineError::Ssh(
                sshboard_ssh::SshError::NotShown(format!("{acked}/{expected}")),
            )),
        }
    }

    /// 操作の宛先。**開いていなければ、そう言う。**
    async fn session(&self) -> Result<Arc<SshSession>, EngineError> {
        let held = self.held.lock().await;
        held.active
            .as_ref()
            .and_then(|id| held.live.get(id))
            .map(|l| Arc::clone(&l.session))
            .ok_or(EngineError::NotConnected)
    }

    // --- 読み取り -----------------------------------------------------------

    pub async fn list_dir(&self, actor: Actor, path: &str) -> Result<Vec<DirEntry>, EngineError> {
        Ok(self.session().await?.list_dir(actor, path).await?)
    }

    pub async fn read_file(&self, actor: Actor, path: &str) -> Result<Vec<u8>, EngineError> {
        Ok(self.session().await?.read_file(actor, path).await?)
    }

    pub async fn exec(&self, actor: Actor, command: &str) -> Result<String, EngineError> {
        Ok(self.session().await?.exec(actor, command).await?)
    }

    /// ログを追う。**GUI へは生・MCP へは素**（Issue 005）。
    pub async fn follow(&self, actor: Actor, path: &str, lines: u32) -> Result<(), EngineError> {
        let session = self.session().await?;
        let stream = Arc::clone(&self.stream);
        Ok(session.follow(actor, path, lines, stream).await?)
    }

    // --- 書き込み（D22） ----------------------------------------------------

    pub async fn ensure_dir(&self, actor: Actor, path: &str) -> Result<(), EngineError> {
        Ok(self.session().await?.ensure_dir(actor, path).await?)
    }

    pub async fn upload_bytes(
        &self,
        actor: Actor,
        path: &str,
        bytes: &[u8],
    ) -> Result<u64, EngineError> {
        Ok(self.session().await?.upload(actor, path, bytes).await?)
    }

    /// 手元のファイルを 1 つ上げる。
    ///
    /// **中身をここで読みます。**巨大なファイルは丸ごとメモリに載ります。
    /// 分割送信が要る大きさに当たったら、そのとき実測して直します（YAGNI）。
    pub async fn upload_file(
        &self,
        actor: Actor,
        local: &Path,
        remote: &str,
    ) -> Result<u64, EngineError> {
        let bytes = tokio::fs::read(local)
            .await
            .map_err(|error| EngineError::Local(format!("{}: {error}", local.display())))?;
        self.upload_bytes(actor, remote, &bytes).await
    }

    // --- 接続一覧 -----------------------------------------------------------

    fn entry(&self, id: &str) -> Result<ConnectionEntry, EngineError> {
        let connections = Connections::load_or_empty(&self.connections_path)
            .map_err(|error| EngineError::Connections(error.to_string()))?;
        connections
            .get(id)
            .cloned()
            .ok_or_else(|| EngineError::UnknownConnection(id.to_owned()))
    }

    /// 認証のやり方を決める。**秘密はここでしか触りません**（D11 / D14）。
    fn auth_for(
        &self,
        entry: &ConnectionEntry,
        passphrase: Option<String>,
    ) -> Result<Auth, EngineError> {
        let Some(path) = entry.key_path.clone() else {
            // 鍵の指定が無ければ ssh-agent。**製品がパスフレーズを持たない一番良い形**（D11）。
            return Ok(Auth::Agent);
        };

        // 人がその場で入れたものが最優先。無ければ OS ストアの参照名から引く。
        let stored = match (&passphrase, &entry.keyring_passphrase_ref) {
            (Some(_), _) => None,
            (None, Some(reference)) => SecretStore::new(KEYRING_SERVICE).get(reference).ok(),
            (None, None) => None,
        };

        let secret = passphrase.or(stored);
        if secret.is_none() && key_needs_passphrase(&path) {
            return Err(EngineError::PassphraseNeeded {
                id: entry.id.clone(),
            });
        }
        Ok(Auth::Key {
            path,
            passphrase: secret,
        })
    }
}

/// `known_hosts` を読む。**読めなくても繋げる**（指紋の固定があるため）。
fn read_known_hosts(explicit: Option<&str>) -> String {
    let path = match explicit {
        Some(path) => PathBuf::from(path),
        None => match std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            Some(home) => PathBuf::from(home).join(".ssh").join("known_hosts"),
            None => return String::new(),
        },
    };
    std::fs::read_to_string(path).unwrap_or_default()
}

/// 鍵が暗号化されているか。**中身は見ますが、どこにも渡しません。**
///
/// 判定を誤ると「パスフレーズを聞かずに失敗する」だけなので、
/// 見出しだけを見る素朴な判定にしています。
fn key_needs_passphrase(path: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        // 読めないなら、ここでは判断しない。**繋ぎに行って正直に失敗させる。**
        return false;
    };
    if text.contains("Proc-Type: 4,ENCRYPTED") || text.contains("ENCRYPTED PRIVATE KEY") {
        return true;
    }
    // OpenSSH 形式は本文の中に暗号方式が入る。`none` なら素のまま。
    text.contains("OPENSSH PRIVATE KEY") && !openssh_body_is_unencrypted(&text)
}

fn openssh_body_is_unencrypted(text: &str) -> bool {
    use std::sync::OnceLock;
    // 本文の先頭は base64 で "openssh-key-v1\0" + 暗号方式名。
    // 暗号方式が `none` の鍵は、この並びが必ず同じ前置きになる。
    static PREFIX: OnceLock<String> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| "b3BlbnNzaC1rZXktdjEAAAAABG5vbmU".to_string());
    text.lines().any(|line| line.starts_with(prefix.as_str()))
}
