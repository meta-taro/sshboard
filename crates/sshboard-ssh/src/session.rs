//! SSH 1 本。**この上に `sftp` と `exec` を載せます。**
//!
//! **2 本目を張りません**（PRD §4-1「裏で見えないセッションを張らない」）。
//! **すべての操作が帯に出ます**（PRD §4-2）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{keys, ChannelMsg};
use sshboard_band::{Actor, Band};
use sshboard_diag::{Diagnostics, Stage};

use crate::hostkey::{decide, fingerprint, fingerprints_for, SeenHostKey, Trust};
use crate::write_scope::{Refusal, WriteScope};

const TIMEOUT: Duration = Duration::from_secs(15);

/// 繋げなかった理由。**握り潰さない。**
#[derive(Debug)]
pub enum SshError {
    /// **ホスト鍵が信用できない。**初見か、登録と食い違う。
    UntrustedHost {
        seen: SeenHostKey,
        trust: Trust,
    },
    Connect(String),
    Authenticate(String),
    Command(String),
    /// 帯が受け取りを返さなかった。**見えないまま実行しない**（D16）。
    NotShown(String),
    /// **AI の書き込みを囲いが断った**（D22）。サーバーへは届いていない。
    WriteRefused(Refusal),
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::UntrustedHost { seen, trust } => match trust {
                Trust::Mismatch { expected } => write!(
                    f,
                    "ホスト鍵が登録と違います（{} / 見えたもの {}・登録 {expected}）",
                    seen.algorithm, seen.fingerprint
                ),
                _ => write!(
                    f,
                    "初めて見るホストです（{} / {}）。確かめて登録してください",
                    seen.algorithm, seen.fingerprint
                ),
            },
            SshError::Connect(detail) => write!(f, "繋がりません: {detail}"),
            SshError::Authenticate(detail) => write!(f, "認証が通りません: {detail}"),
            SshError::Command(detail) => write!(f, "コマンドが通りません: {detail}"),
            SshError::NotShown(detail) => write!(f, "画面へ出せませんでした: {detail}"),
            SshError::WriteRefused(why) => write!(f, "書き込みを断りました: {why}"),
        }
    }
}

impl std::error::Error for SshError {}

/// 認証のやり方。**パスワード認証を用意しません**（履歴と記憶に残る経路を作らない）。
pub enum Auth {
    /// ssh-agent へ委譲する。**パスフレーズを製品が受け取らない**（D11・推奨）。
    Agent,
    /// 鍵ファイル。パスフレーズは**人がその場で入れたものを渡す**。
    Key {
        path: String,
        passphrase: Option<String>,
    },
}

/// 繋ぐときに要るもの。**接続先はここにしか無い。**
pub struct Target {
    /// 接続の識別子。**記録に出せる唯一の名前**（ホスト名は出せない・PRD §8）。
    pub id: Option<String>,
    pub host: String,
    pub port: u16,
    pub user: String,
    /// 登録された指紋。**あればこちらを優先する。**
    pub pinned_fingerprint: Option<String>,
    /// `known_hosts` の中身。読めなければ空でよい。
    pub known_hosts: String,
    /// **AI が書いてよい範囲**（D22）。既定は `Denied` ＝ AI は書けない。
    pub write_scope: WriteScope,
}

/// ホスト鍵を見て覚えるだけのハンドラ。**判断は接続側が行う。**
struct Watcher {
    seen: Arc<Mutex<Option<SeenHostKey>>>,
}

impl client::Handler for Watcher {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        if let PublicKeyOrCertificate::PublicKey { key, .. } = server_public_key {
            let seen = SeenHostKey {
                algorithm: key.algorithm().to_string(),
                fingerprint: fingerprint(key.to_bytes().unwrap_or_default().as_slice()),
            };
            if let Ok(mut held) = self.seen.lock() {
                *held = Some(seen);
            }
        }
        // **ここでは通す。**判断は接続側で、記録した指紋と known_hosts に照らして行う。
        // 通したあとに信用できなければ、**セッションを捨てる。**
        Ok(true)
    }
}

/// 繋がっている 1 本。
pub struct SshSession {
    handle: Handle<Watcher>,
    band: Band,
    host_key: SeenHostKey,
    write_scope: WriteScope,
}

impl SshSession {
    /// 繋いで、ホスト鍵を確かめる。
    ///
    /// **信用できないホストとは、繋いだあとでもセッションを捨てます。**
    pub async fn connect(
        target: &Target,
        auth: &Auth,
        band: Band,
        diag: &Diagnostics,
    ) -> Result<Self, SshError> {
        let id = target.id.as_deref();
        diag.info(
            Stage::Reach,
            id,
            format!("繋ぎに行きます（ポート {}）", target.port),
        );

        let seen = Arc::new(Mutex::new(None));
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(TIMEOUT),
            ..Default::default()
        });

        let mut handle = client::connect(
            config,
            (target.host.as_str(), target.port),
            Watcher {
                seen: Arc::clone(&seen),
            },
        )
        .await
        .map_err(|error| {
            diag.error(
                Stage::Reach,
                id,
                format!("繋がりません: {error}"),
                "相手が動いているか、ポート番号と経路（VPN・許可 IP）を確かめてください",
            );
            SshError::Connect(error.to_string())
        })?;
        diag.info(Stage::Reach, id, "繋がりました");

        let host_key = seen
            .lock()
            .ok()
            .and_then(|held| held.clone())
            .ok_or_else(|| SshError::Connect("ホスト鍵を受け取れませんでした".into()))?;

        let known = fingerprints_for(&target.known_hosts, &target.host, target.port);
        let trust = decide(&host_key, target.pinned_fingerprint.as_deref(), &known);

        if !trust.is_acceptable() {
            let (what, hint) = match &trust {
                Trust::Mismatch { .. } => (
                    "ホスト鍵が登録と違います",
                    "サーバーを建て直したのでなければ、すり替えの疑いがあります。\
                     建て直したと分かっているなら、接続の登録から指紋を消してください",
                ),
                _ => (
                    "初めて見るホストです",
                    "画面に出た指紋を確かめて登録してください（サーバー上の \
                     `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` と見比べます）",
                ),
            };
            diag.error(
                Stage::HostKey,
                id,
                format!(
                    "{what}（{} / {}）",
                    host_key.algorithm, host_key.fingerprint
                ),
                hint,
            );

            // **繋いだあとでも捨てる。**信用できないまま使わない。
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "")
                .await;
            return Err(SshError::UntrustedHost {
                seen: host_key,
                trust,
            });
        }

        diag.info(
            Stage::HostKey,
            id,
            format!(
                "ホスト鍵を確かめました（{} / {}）",
                host_key.algorithm, host_key.fingerprint
            ),
        );

        authenticate(&mut handle, &target.user, auth, diag, id).await?;
        diag.info(Stage::Auth, id, "認証が通りました");

        Ok(Self {
            handle,
            band,
            host_key,
            write_scope: target.write_scope.clone(),
        })
    }

    /// 繋がっている相手のホスト鍵。**人が known_hosts と突き合わせるため。**
    pub fn host_key(&self) -> &SeenHostKey {
        &self.host_key
    }

    /// この接続で AI が書いてよい範囲。**人へ見せるため。**
    pub fn write_scope(&self) -> &WriteScope {
        &self.write_scope
    }

    /// コマンドを実行し、出力を返す。**先に帯へ出す**（D16）。
    pub async fn exec(&self, actor: Actor, command: &str) -> Result<String, SshError> {
        self.show(actor, &format!("$ {command}")).await?;

        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .exec(true, command)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        let mut out = Vec::new();
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { ref data } => out.extend_from_slice(data),
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// 帯へ載せ、画面が受け取るまで待つ（D16）。
    async fn show(&self, actor: Actor, text: &str) -> Result<(), SshError> {
        let delivery = self.band.record(actor, text);
        match delivery.wait_acked(Duration::from_secs(2)).await {
            sshboard_band::DeliveryOutcome::Delivered => Ok(()),
            sshboard_band::DeliveryOutcome::TimedOut { acked, expected } => {
                Err(SshError::NotShown(format!("{acked}/{expected}")))
            }
        }
    }
}

async fn authenticate(
    handle: &mut Handle<Watcher>,
    user: &str,
    auth: &Auth,
    diag: &Diagnostics,
    id: Option<&str>,
) -> Result<(), SshError> {
    let result = match auth {
        // **agent へ委譲する道は OS ごとに違う**（D11）。分岐は下の関数に閉じている。
        Auth::Agent => {
            diag.info(Stage::Auth, id, "ssh-agent の鍵で試します");
            return authenticate_with_agent(handle, user, diag, id).await;
        }
        Auth::Key { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|error| SshError::Authenticate(format!("鍵を読めません: {error}")))?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|error| SshError::Authenticate(error.to_string()))?
                .flatten();
            handle
                .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                .await
                .map_err(|error| SshError::Authenticate(error.to_string()))?
        }
    };

    if result.success() {
        Ok(())
    } else {
        Err(SshError::Authenticate(
            "鍵が受け付けられませんでした".into(),
        ))
    }
}

/// ssh-agent へ委譲して認証する。**製品は鍵にもパスフレーズにも触りません**（D11）。
///
/// **agent への繋ぎ方は OS で違います。**
/// Unix は `SSH_AUTH_SOCK`。Windows は OpenSSH の名前付きパイプで、
/// それが無ければ Pageant（PuTTY）を試します。
/// **Pageant を見るのは実利です**（D19）。この層の利用者は鍵を `.ppk` で持っていて、
/// Pageant に入っていればそのまま繋がります。
#[cfg(unix)]
async fn authenticate_with_agent(
    handle: &mut Handle<Watcher>,
    user: &str,
    diag: &Diagnostics,
    id: Option<&str>,
) -> Result<(), SshError> {
    let mut agent = keys::agent::client::AgentClient::connect_env()
        .await
        .map_err(|error| {
            diag.error(
                Stage::Auth,
                id,
                format!("ssh-agent に繋げません: {error}"),
                "ssh-agent が動いていないか、SSH_AUTH_SOCK が渡っていません。\
                 端末で `ssh-add -l` が通るか確かめてください",
            );
            SshError::Authenticate(format!("ssh-agent: {error}"))
        })?;
    try_agent_identities(handle, user, &mut agent, diag, id).await
}

#[cfg(windows)]
async fn authenticate_with_agent(handle: &mut Handle<Watcher>, user: &str) -> Result<(), SshError> {
    /// Windows の OpenSSH agent が待っている場所。**固定の名前。**
    const OPENSSH_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

    // 1. Windows の OpenSSH agent
    let openssh_error =
        match keys::agent::client::AgentClient::connect_named_pipe(OPENSSH_PIPE).await {
            Ok(mut agent) => return try_agent_identities(handle, user, &mut agent, diag, id).await,
            Err(error) => error,
        };

    // 2. Pageant（PuTTY）。**`.ppk` を持っている人がここに居る**（D19）。
    match keys::agent::client::AgentClient::connect_pageant().await {
        Ok(mut agent) => try_agent_identities(handle, user, &mut agent, diag, id).await,
        // **どちらが駄目だったかを両方出す。**片方だけだと人が探せない。
        Err(pageant_error) => {
            diag.error(
                Stage::Auth,
                id,
                format!(
                    "ssh-agent に繋げません（OpenSSH: {openssh_error} / Pageant: {pageant_error}）"
                ),
                "Windows の OpenSSH エージェントを起動するか、Pageant に鍵を入れてください",
            );
            Err(SshError::Authenticate(format!(
                "ssh-agent に繋げません（OpenSSH: {openssh_error} / Pageant: {pageant_error}）"
            )))
        }
    }
}

/// agent が持っている鍵を順に試す。**通ったら、そこで終わり。**
///
/// **どの鍵を試したかを指紋で記録します。**指紋は秘密ではありません。
/// **コメントは記録しません** — パスやメールアドレスが入っているためです。
async fn try_agent_identities<S>(
    handle: &mut Handle<Watcher>,
    user: &str,
    agent: &mut keys::agent::client::AgentClient<S>,
    diag: &Diagnostics,
    id: Option<&str>,
) -> Result<(), SshError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let identities = agent.request_identities().await.map_err(|error| {
        diag.error(
            Stage::Auth,
            id,
            format!("ssh-agent から鍵の一覧を取れません: {error}"),
            "`ssh-add -l` が通るか確かめてください",
        );
        SshError::Authenticate(format!("ssh-agent: {error}"))
    })?;

    let mut tried = Vec::new();
    for identity in identities {
        let keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
            continue;
        };
        let shown = key.fingerprint(Default::default()).to_string();
        tried.push(shown.clone());

        let attempt = handle
            .authenticate_publickey_with(user, key, None, agent)
            .await
            .map_err(|error| {
                diag.error(
                    Stage::Auth,
                    id,
                    format!("鍵を出せません（{shown}）: {error}"),
                    "接続が途中で切れた可能性があります。もう一度試してください",
                );
                SshError::Authenticate(error.to_string())
            })?;

        if attempt.success() {
            diag.info(Stage::Auth, id, format!("鍵が通りました（{shown}）"));
            return Ok(());
        }
        diag.info(Stage::Auth, id, format!("受け付けられません（{shown}）"));
    }

    // **「鍵が無い」と「全部弾かれた」を混ぜない。**人が次にやることが違う。
    if tried.is_empty() {
        diag.error(
            Stage::Auth,
            id,
            "ssh-agent に鍵が 1 本も入っていません",
            "`ssh-add <鍵のパス>` で入れるか、接続の登録に鍵のパスを書いてください",
        );
        return Err(SshError::Authenticate(
            "ssh-agent に鍵がありません（ssh-add してください）".into(),
        ));
    }

    diag.error(
        Stage::Auth,
        id,
        format!(
            "ssh-agent の鍵 {} 本とも受け付けられませんでした（{}）",
            tried.len(),
            tried.join(" / ")
        ),
        "この相手に対応する鍵が ssh-agent に入っていません。\
         `ssh-add <鍵のパス>` で足すか、接続の登録に鍵のパスを書いてください",
    );
    Err(SshError::Authenticate(format!(
        "ssh-agent の鍵 {} 本とも受け付けられませんでした。\
         この相手に対応する鍵が入っていない可能性が高いです",
        tried.len()
    )))
}

/// `sftp` で見えたもの 1 件。**中身は持ちません。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

impl SshSession {
    /// 同じセッションの上に `sftp` を開く。**2 本目の接続を張りません。**
    async fn sftp(&self) -> Result<russh_sftp::client::SftpSession, SshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| SshError::Command(error.to_string()))
    }

    /// ディレクトリの一覧。**先に帯へ出す**（D16）。
    pub async fn list_dir(&self, actor: Actor, path: &str) -> Result<Vec<DirEntry>, SshError> {
        self.show(actor, &format!("ls {path}")).await?;

        let sftp = self.sftp().await?;
        let entries = sftp
            .read_dir(path)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        Ok(entries
            .map(|entry| DirEntry {
                name: entry.file_name(),
                is_dir: entry.file_type().is_dir(),
                size: entry.metadata().size.unwrap_or(0),
            })
            .collect())
    }

    /// ファイルを丸ごと読む。**バイト列のまま返します。**
    ///
    /// **ここで文字コードを決めません。**EUC-JP のログが実在するので
    /// （Issue 002・手元のテスト用サーバーでも再現済み）、
    /// **変換するかどうかは呼び出し側が決めます。**
    pub async fn read_file(&self, actor: Actor, path: &str) -> Result<Vec<u8>, SshError> {
        use tokio::io::AsyncReadExt;

        self.show(actor, &format!("read {path}")).await?;

        let sftp = self.sftp().await?;
        let mut file = sftp
            .open(path)
            .await
            .map_err(|e| SshError::Command(e.to_string()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        Ok(bytes)
    }

    /// **書き込みの入口はここ 1 か所**（D22）。
    ///
    /// 囲いがかかるのは **AI だけ**です。人は普通の SFTP クライアントとして自由に使えます
    /// （PRD §3「人（GUI）の側は制限しない」）。
    fn allow_write(&self, actor: Actor, path: &str, as_dir: bool) -> Result<(), SshError> {
        if actor == Actor::Human {
            return Ok(());
        }
        let verdict = if as_dir {
            self.write_scope.permits_dir(path)
        } else {
            self.write_scope.permits(path)
        };
        verdict.map_err(SshError::WriteRefused)
    }

    /// ディレクトリを（親ごと）用意する。**既に在るものは触りません。**
    ///
    /// AI のときは、囲いの中に入る階層だけを作ります。囲いの外の階層は
    /// **既に在る前提**で飛ばし、無ければ下の階層の作成が正直に失敗します。
    pub async fn ensure_dir(&self, actor: Actor, path: &str) -> Result<(), SshError> {
        self.allow_write(actor, path, true)?;
        self.show(actor, &format!("mkdir -p {path}")).await?;

        let sftp = self.sftp().await?;
        for dir in ancestors(path) {
            if self.allow_write(actor, &dir, true).is_err() {
                // 囲いの外の階層。**作らないが、断りもしない**（既に在るはずのもの）。
                continue;
            }
            match sftp.metadata(&dir).await {
                Ok(meta) if meta.is_dir() => continue,
                Ok(_) => {
                    return Err(SshError::Command(format!(
                        "{dir} は既にファイルとして在ります"
                    )))
                }
                Err(_) => sftp
                    .create_dir(&dir)
                    .await
                    .map_err(|error| SshError::Command(format!("{dir}: {error}")))?,
            }
        }
        Ok(())
    }

    /// ファイルを 1 つ上げる。**書く前に帯へ出します**（D16）。
    ///
    /// 中身は**バイト列のまま**渡します。ここで文字コードを決めません。
    pub async fn upload(&self, actor: Actor, path: &str, bytes: &[u8]) -> Result<u64, SshError> {
        use tokio::io::AsyncWriteExt;

        // **サーバーへ触る前に断る。**断ったのに 0 バイトのファイルが残る、を起こさない。
        self.allow_write(actor, path, false)?;
        self.show(actor, &format!("upload {path} ({} bytes)", bytes.len()))
            .await?;

        let sftp = self.sftp().await?;
        let mut file = sftp
            .create(path)
            .await
            .map_err(|error| SshError::Command(format!("{path}: {error}")))?;
        file.write_all(bytes)
            .await
            .map_err(|error| SshError::Command(format!("{path}: {error}")))?;
        file.shutdown()
            .await
            .map_err(|error| SshError::Command(format!("{path}: {error}")))?;
        Ok(bytes.len() as u64)
    }

    /// ログを追う。**同じ 1 本の出力を、GUI へは生・MCP へは素で流します**（Issue 005）。
    ///
    /// 人が止めたら（`OutputStream::stop`）、**その場で追うのをやめます**（PRD §4-3）。
    pub async fn follow(
        &self,
        actor: Actor,
        path: &str,
        lines: u32,
        into: Arc<sshboard_stream::OutputStream>,
    ) -> Result<(), SshError> {
        // 引数を組み立てるのは**こちら**で、外から任意の文字列を渡させない（D3）。
        let command = format!("tail -n {lines} -f {}", shell_quote(path));
        self.show(actor, &format!("$ {command}")).await?;

        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;
        channel
            .exec(true, command)
            .await
            .map_err(|error| SshError::Command(error.to_string()))?;

        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { ref data } => {
                    // 人が止めたら、そこで終わり（PRD §4-3）。
                    if into.push(data).is_err() {
                        break;
                    }
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        let _ = channel.close().await;
        Ok(())
    }
}

/// パスを 1 語として渡す。**`run_command` を作らないための下ごしらえ**（D3）。
///
/// 単引用符で囲み、中の単引用符だけを閉じ直す。
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// `/a/b/c` を `["/", "/a", "/a/b", "/a/b/c"]` へ。**浅い順。**
fn ancestors(path: &str) -> Vec<String> {
    let mut out = vec!["/".to_string()];
    let mut current = String::new();
    for component in path.split('/').filter(|c| !c.is_empty()) {
        current.push('/');
        current.push_str(component);
        out.push(current.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::ancestors;

    #[test]
    fn ancestors_are_listed_from_the_root_downwards() {
        assert_eq!(
            ancestors("/srv/app/release"),
            vec!["/", "/srv", "/srv/app", "/srv/app/release"]
        );
    }

    #[test]
    fn a_trailing_or_doubled_slash_does_not_produce_an_empty_level() {
        // 空の階層が 1 つ混じると `mkdir ""` を投げて、読みにくい失敗になる。
        assert_eq!(ancestors("/srv//app/"), vec!["/", "/srv", "/srv/app"]);
        assert_eq!(ancestors("/"), vec!["/"]);
    }
}
