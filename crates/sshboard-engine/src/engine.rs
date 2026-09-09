//! 開いているものを**全部**持ち、すべての操作をここへ集める（D25）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_connections::{ConnectionEntry, Connections};
use sshboard_credentials::SecretStore;
use sshboard_diag::{Diagnostics, Stage};
use sshboard_readonly::{Allowlist, ReadonlyCommand, Refusals};
use sshboard_ssh::{
    inspect_key, Auth, Console, DirEntry, FileFacts, KeyFacts, KeyFormat, KeyVerdict, Ran,
    SshSession, Target, WriteScope,
};
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

/// 端末を握っている側と、その 1 本（D29）。
///
/// **ロックはここ 1 か所だけが持ちます。**画面と MCP が別々に持つと、
/// 必ず食い違います（D25 で実際に食い違って気づきました）。
#[derive(Default)]
struct ConsoleSlot {
    console: Option<Console>,
    holder: Option<Actor>,
    /// **どの接続の端末か**（D25 と噛み合わせる）。
    ///
    /// これを持たないと、タブを移したあとの打鍵が**前のサーバーへ行き続け**、
    /// 画面は別の接続を向いたままになります。**識別子だけ**を持ちます
    /// （ホスト名は持たない・CLAUDE.md 禁止事項 4）。
    connection: Option<String>,
    /// **AI が「使いたい」と言っている**（D42）。人が答えるまで残ります。
    ///
    /// 積み上げません。**何度頼まれても、人に出る問いは 1 つ**です
    /// （催促で人を疲れさせると、いずれ中身を見ずに許すようになります）。
    request: Option<Actor>,
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
    /// 端末の 1 本と、握っている側（D29）。
    console: Mutex<ConsoleSlot>,
    /// 誰が握っているかを配る。**画面が知らないまま AI が打っている、を作らない。**
    console_changed: watch::Sender<Option<Actor>>,
    /// **AI からの頼みを画面へ押し出す**（D42）。
    /// 出せない問いは、無いのと同じです。
    console_request_changed: watch::Sender<Option<Actor>>,
    /// **AI が繋ごうとして、パスフレーズで止まった接続**（Issue #13）。
    ///
    /// 識別子だけを持ちます。**パスフレーズ本体はここへ来ません**（D14）——
    /// 人が入れた値は、人の経路（`connect`）へ直接渡ります。
    passphrase_request: watch::Sender<Option<String>>,
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
        let (console_changed, _) = watch::channel(None);
        let (console_request_changed, _) = watch::channel(None);
        let (passphrase_request, _) = watch::channel(None);
        Self {
            band,
            diag,
            stream,
            connections_path,
            held: Mutex::new(Held::default()),
            console: Mutex::new(ConsoleSlot::default()),
            console_changed,
            console_request_changed,
            passphrase_request,
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

    /// **いま、どの接続がパスフレーズ待ちか**（Issue #13）。
    ///
    /// 画面はこれを見て問いを出します。**識別子だけ**です（PRD §8）。
    pub fn passphrase_request(&self) -> Option<String> {
        self.passphrase_request.borrow().clone()
    }

    /// 頼みの変化を受け取る口。**押し出さないと、人は気づけません。**
    pub fn subscribe_passphrase_request(&self) -> watch::Receiver<Option<String>> {
        self.passphrase_request.subscribe()
    }

    /// 問いを畳む。**人が入れたときも、断ったときも通ります。**
    pub fn clear_passphrase_request(&self) {
        let _ = self.passphrase_request.send(None);
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
        let auth = match self.auth_for(&entry, passphrase) {
            Ok(auth) => auth,
            // **出せない画面を案内しない**（Issue #13）。
            //
            // 実機の指摘: 「AI から `connect` を呼ぶと『画面で人が入れてください』と
            // 返るが、**アプリ側にプロンプトが一切出ない**。案内された側は詰みます」。
            //
            // パスフレーズの問いは人の経路にしか無かったので、**AI が頼めば
            // 人の画面に出る**ようにします（D42 と同じ形）。
            Err(EngineError::PassphraseNeeded { id }) => {
                if actor != Actor::Human {
                    let _ = self.passphrase_request.send(Some(id.clone()));
                }
                return Err(EngineError::PassphraseNeeded { id });
            }
            Err(other) => return Err(other),
        };

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
        // **繋がったら問いを畳む**（Issue #13）。
        // 人が入れて繋がったのに問いが残ると、**もう一度入れさせる**ことになります。
        self.clear_passphrase_request();
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

    // --- 端末（D29） --------------------------------------------------------

    /// 誰が端末を握っているか。**画面はこれを見て入力を締めます。**
    pub async fn console_holder(&self) -> Option<Actor> {
        self.console.lock().await.holder
    }

    /// **どの接続の端末か。**画面にも MCP にも、これを添えて出します。
    pub async fn console_connection(&self) -> Option<String> {
        self.console.lock().await.connection.clone()
    }

    /// 握っている側の変化を受け取る口。**画面が知らないまま AI が打っている、を作らない。**
    pub fn subscribe_console(&self) -> watch::Receiver<Option<Actor>> {
        self.console_changed.subscribe()
    }

    /// 端末を開いて握る（D29）。**既に誰かが握っていれば断ります。**
    pub async fn console_open(
        &self,
        actor: Actor,
        cols: u32,
        rows: u32,
    ) -> Result<(), EngineError> {
        // **AI が握るには、人の許可が要る**（D42）。**サーバーへ行く前に**尋ねます。
        self.ask_first(actor).await?;

        // **開けなかったことを残す**（Issue #10）。
        //
        // 実機で端末が繋がらなかったとき、記録に残っていたのは接続の 4 行だけで、
        // **端末の行は 1 本もありませんでした。追えない失敗は、直せない失敗**です。
        let Some(target) = self.active().await.map(|open| open.id) else {
            self.diag.error(
                Stage::Exec,
                None,
                "端末を開けません（繋がっていません）",
                "先に接続を開いてください",
            );
            return Err(EngineError::NotConnected);
        };
        {
            let slot = self.console.lock().await;
            if let Some(holder) = slot.holder {
                // 同じ側が開き直すのは、握り直しとして通す。
                if holder != actor {
                    self.diag.error(
                        Stage::Exec,
                        Some(&target),
                        format!("端末を開けません（{}が握っています）", who(holder)),
                        "人は画面の［取り返す］でいつでも取り返せます",
                    );
                    return Err(held_by(holder));
                }
            }
            // **別の接続では開き直さない**（D25）。
            // 黙って乗り換えると、打鍵がどちらへ行くのか分からなくなる。
            if let Some(open_on) = slot.connection.as_deref() {
                if open_on != target {
                    self.diag.error(
                        Stage::Exec,
                        Some(&target),
                        format!("端末を開けません（{open_on} で開いています）"),
                        "先に［止める］を押してください",
                    );
                    return Err(EngineError::ConsoleOnOtherConnection {
                        id: open_on.to_owned(),
                    });
                }
            }
        }

        let session = self.session().await?;
        let console = session
            .open_console(actor, cols, rows, Arc::clone(&self.stream))
            .await?;

        let mut slot = self.console.lock().await;
        // 開いている間に別の側が入っていたら、開いたものは捨てて断る。
        if let Some(holder) = slot.holder {
            if holder != actor {
                drop(slot);
                console.close().await;
                return Err(held_by(holder));
            }
        }
        if let Some(previous) = slot.console.take() {
            previous.close().await;
        }
        slot.console = Some(console);
        slot.holder = Some(actor);
        slot.connection = Some(target.clone());
        drop(slot);

        // **開けたことも残す。**失敗だけ残すと、「開いたのに映らない」を追えません
        // （実機がまさにその形でした・Issue #10）。
        self.diag.info(
            Stage::Exec,
            Some(&target),
            format!("端末を開きました（{}・{cols}×{rows}）", who(actor)),
        );
        let _ = self.console_changed.send(Some(actor));
        Ok(())
    }

    /// 打ち込む。**握っている側だけ**（D29）。
    ///
    /// **通った打鍵は記録しません。**1 キーずつ残すと記録が溢れ、
    /// **打った中身がそのまま残る**ことにもなります（パスワードを打つ人が居ます）。
    /// 残すのは**断った事実だけ**です — Issue #10 の「入力が届かない」を追う材料。
    pub async fn console_type(&self, actor: Actor, bytes: &[u8]) -> Result<(), EngineError> {
        let slot = self.console.lock().await;
        match slot.holder {
            None => {
                drop(slot);
                self.diag.error(
                    Stage::Exec,
                    None,
                    "打鍵を断りました（端末が開いていません）",
                    "先に端末を開いてください",
                );
                Err(EngineError::ConsoleNotOpen)
            }
            Some(holder) if holder != actor => {
                drop(slot);
                self.diag.error(
                    Stage::Exec,
                    None,
                    format!(
                        "打鍵を断りました（{}が打ち、{}が握っています）",
                        who(actor),
                        who(holder)
                    ),
                    "同時に触れるのは 1 人です。人は［取り返す］で取り返せます",
                );
                Err(held_by(holder))
            }
            Some(_) => {
                let console = slot.console.as_ref().ok_or(EngineError::ConsoleNotOpen)?;
                Ok(console.type_in(bytes).await?)
            }
        }
    }

    /// 窓の大きさを伝える。**握っていなくても通す**（見ている側の画面も追従するため）。
    pub async fn console_resize(&self, cols: u32, rows: u32) -> Result<(), EngineError> {
        let slot = self.console.lock().await;
        let console = slot.console.as_ref().ok_or(EngineError::ConsoleNotOpen)?;
        Ok(console.resize(cols, rows).await?)
    }

    /// 握りを取り返す。**人は常に勝ちます**（D29）。
    ///
    /// AI は、誰も握っていないか自分が握っているときだけ取れます。
    /// **AI が人から奪える形にしない。**
    pub async fn console_take(&self, actor: Actor) -> Result<(), EngineError> {
        // **AI が握るには、人の許可が要る**（D42）。
        self.ask_first(actor).await?;
        let mut slot = self.console.lock().await;
        match slot.holder {
            Some(holder) if holder != actor && actor != Actor::Human => {
                drop(slot);
                self.diag.error(
                    Stage::Exec,
                    None,
                    format!("握りを渡しませんでした（{}が握っています）", who(holder)),
                    "AI は人から奪えません（D29）",
                );
                Err(held_by(holder))
            }
            previous => {
                slot.holder = Some(actor);
                drop(slot);
                // **握りが移ったことを残す。**誰が打っていたのかが後から読めないと、
                // 「打てなくなった」の切り分けができません（Issue #10）。
                self.diag.info(
                    Stage::Exec,
                    None,
                    match previous {
                        Some(holder) => {
                            format!("握りが{}から{}へ移りました", who(holder), who(actor))
                        }
                        None => format!("{}が握りました", who(actor)),
                    },
                );
                let _ = self.console_changed.send(Some(actor));
                Ok(())
            }
        }
    }

    /// **AI が端末を使いたいと言っているか**（D42）。画面はこれを見て問いを出します。
    pub async fn console_request(&self) -> Option<Actor> {
        self.console.lock().await.request
    }

    /// 頼みの変化を受け取る口。**押し出さないと、人は気づけません。**
    pub fn subscribe_console_request(&self) -> watch::Receiver<Option<Actor>> {
        self.console_request_changed.subscribe()
    }

    /// 人が答える（D42）。**答えられるのは人だけ。**
    ///
    /// 自分で自分を許可できたら、許可の意味がありません。
    /// 許したら、その場で握りが移ります。**もう［止める］を押させません。**
    pub async fn console_answer(&self, actor: Actor, allow: bool) -> Result<(), EngineError> {
        if actor != Actor::Human {
            return Err(EngineError::ConsoleApprovalNeeded);
        }
        let mut slot = self.console.lock().await;
        let Some(asked_by) = slot.request.take() else {
            // 問いが無いのに答えた。**同じ状態へ向かうので失敗にしません。**
            drop(slot);
            let _ = self.console_request_changed.send(None);
            return Ok(());
        };
        let moved = if allow {
            slot.holder = Some(asked_by);
            true
        } else {
            false
        };
        drop(slot);

        self.diag.info(
            Stage::Exec,
            None,
            if moved {
                format!("人が許可しました。握りが{}へ移りました", who(asked_by))
            } else {
                format!("人が断りました。{}は握れません", who(asked_by))
            },
        );
        let _ = self.console_request_changed.send(None);
        if moved {
            let _ = self.console_changed.send(Some(asked_by));
        }
        Ok(())
    }

    /// **AI は、握る前に人へ頼む**（D42）。人はそのまま通ります。
    ///
    /// 実機の指摘（2026-09-06）から入れました。
    ///
    /// > AI に端末を渡すときに「止める」を押さないといけません。
    /// > これだと **AI からのアクションが分からない**ので、
    /// > 「AI が操作をするために許可しますか」みたいなアラートで人に気づかせないと。
    ///
    /// **握り手が居なくても頼ませます。**「居ないなら黙って取れる」だと、
    /// **人は AI が触ったことに気づけません。**
    async fn ask_first(&self, actor: Actor) -> Result<(), EngineError> {
        if actor == Actor::Human {
            return Ok(());
        }
        let mut slot = self.console.lock().await;
        // すでに握っているなら、頼み直させません（打鍵のたびに問いが出ます）。
        if slot.holder == Some(actor) {
            return Ok(());
        }
        // **催促を積み上げない。**何度呼ばれても、人に出る問いは 1 つ。
        let is_new = slot.request != Some(actor);
        slot.request = Some(actor);
        drop(slot);

        if is_new {
            self.diag.info(
                Stage::Exec,
                None,
                format!(
                    "{}が端末を使いたいと言っています。人の答え待ちです",
                    who(actor)
                ),
            );
            let _ = self.console_request_changed.send(Some(actor));
        }
        Err(EngineError::ConsoleApprovalNeeded)
    }

    /// 止める（D29 の停止ボタン）。**握っている側と、人だけ。**
    ///
    /// 帯の受け取りを待ちません。切断と同じ扱いです — **止まらない停止は、
    /// 無い方がまし。**握りも外すので、次の側が開き直せます。
    ///
    /// **`actor` を取るようになりました**（2026-09-06・実機の指摘）。
    /// 以前は誰が呼んだかを見ておらず、**AI が 2 手で人の端末を奪えました。**
    ///
    /// ```text
    /// AI: console_stop   → 人のシェルが落ち、握りが外れる
    /// AI: console_open   → AI が握る
    /// ```
    ///
    /// `console_open` は「他が握っていたら断る」と正しく書いてあったのに、
    /// **ここが裏口**でした。D29 は「人の解除が常に勝つ」と書いていますが、
    /// **守っていたのは片側だけ**です。
    ///
    /// **人は常に勝ちます。**AI が止められるのは、自分が握っている分だけです。
    pub async fn console_stop(&self, actor: Actor) -> Result<(), EngineError> {
        let mut slot = self.console.lock().await;
        // **人は常に勝つ**（D29）。ここは 1 ミリも緩めない。
        if actor != Actor::Human {
            if let Some(holder) = slot.holder {
                if holder != actor {
                    drop(slot);
                    self.diag.error(
                        Stage::Exec,
                        None,
                        format!(
                            "端末を止めさせませんでした（{}が止めようとし、{}が握っています）",
                            who(actor),
                            who(holder)
                        ),
                        "止められるのは、握っている側と人だけです（D29）",
                    );
                    return Err(held_by(holder));
                }
            }
        }
        let console = slot.console.take();
        slot.holder = None;
        slot.connection = None;
        drop(slot);

        if let Some(console) = console {
            console.close().await;
            // **段階は `Exec`。**端末は「繋がったあとのコマンド」で、到達ではありません
            // （他の端末の記録と並べて読めるように揃えました・Issue #10）。
            self.diag.info(
                Stage::Exec,
                None,
                format!("端末を止めました（{}）", who(actor)),
            );
        }
        let _ = self.console_changed.send(None);
        Ok(())
    }

    // --- 読み取り -----------------------------------------------------------

    pub async fn list_dir(&self, actor: Actor, path: &str) -> Result<Vec<DirEntry>, EngineError> {
        Ok(self.session().await?.list_dir(actor, path).await?)
    }

    /// 1 件の属性。**権限と更新日時は「なぜ読めないのか」を説明する材料。**
    pub async fn stat(&self, actor: Actor, path: &str) -> Result<FileFacts, EngineError> {
        Ok(self.session().await?.stat(actor, path).await?)
    }

    pub async fn read_file(&self, actor: Actor, path: &str) -> Result<Vec<u8>, EngineError> {
        Ok(self.session().await?.read_file(actor, path).await?)
    }

    /// コマンドを 1 回打つ。**stderr も終了コードも返します**（握り潰さない）。
    pub async fn exec(&self, actor: Actor, command: &str) -> Result<Ran, EngineError> {
        let ran = self.session().await?.exec(actor, command).await?;

        // **打ったものと、返ってきたものを画面へ出す**（Issue #14 の 2 つ目）。
        //
        // 実機の指摘:
        //
        // > 用途別ツールは実際にサーバーでコマンドを走らせているので、
        // > **端末に出ないほうが不自然**です
        //
        // **そのとおりでした。**`exec` は共有している出力へ 1 バイトも流しておらず、
        // **AI がサーバーで何を見たのかを、人は追えません**でした。
        // 端末（D29）と `tail -f` は流しているのに、ここだけ抜けていました。
        //
        // 別のタブ（操作の記録）へ寄せる案は採りません ——
        // **見る場所が 1 つで済み、人が打った分と AI が打った分が同じ時系列に並ぶ**
        // 方が、「1 つのシェルを共有する」という説明と見え方が一致します。
        //
        // **`tail -f` と混ざります。**出どころの札（D41）は付けていません ——
        // 混ざって困ると分かってから足します（YAGNI）。
        // **いまは「出ない」方が困っている**ので、まず出します。
        self.echo_to_screen(command, &ran);
        Ok(ran)
    }

    /// 打ったものと返ってきたものを、端末の面へ流す。
    ///
    /// **失敗しても握り潰しません**が、**ここで操作を失敗にもしません** ——
    /// 画面へ出せなかったことを理由に、成功したコマンドを失敗扱いにすると、
    /// **AI は同じものを打ち直します**（サーバーで 2 回走ります）。
    fn echo_to_screen(&self, command: &str, ran: &Ran) {
        let mut shown = format!("$ {command}\r\n");
        if !ran.out.is_empty() {
            shown.push_str(&ran.out.replace('\n', "\r\n"));
        }
        if !ran.err.is_empty() {
            shown.push_str(&ran.err.replace('\n', "\r\n"));
        }
        // **終了コードは、0 でないときだけ。**0 を毎回出すと画面が埋まります。
        // **返してこないサーバーもある**ので、そのときは黙ります。
        if let Some(status) = ran.status {
            if status != 0 {
                shown.push_str(&format!("[終了コード {status}]\r\n"));
            }
        }
        if self.stream.push(shown.as_bytes()).is_err() {
            // 人が［止める］を押している。**そういう状態なので、記録もしません。**
            self.diag.info(
                Stage::Exec,
                None,
                "出力が止められているので画面へ出しません",
            );
        }
    }

    // --- 許可リストのコマンド（D3） -----------------------------------------

    /// 許可リストの置き場所。**接続一覧と同じディレクトリ。**
    ///
    /// 2 か所に置くと、人は「どちらを編集したのか」を追えなくなります。
    pub fn readonly_path(&self) -> PathBuf {
        self.beside_connections("readonly.toml")
    }

    /// 断った事実の置き場所（D3 追記）。
    pub fn readonly_refusals_path(&self) -> PathBuf {
        self.beside_connections("readonly-refused.log")
    }

    fn beside_connections(&self, name: &str) -> PathBuf {
        self.connections_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(name)
    }

    /// 人が許したコマンド全部。**製品は既定を 1 本も持ちません**（D3 追記）。
    pub fn readonly_commands(&self) -> Result<Vec<ReadonlyCommand>, EngineError> {
        Ok(self.allowlist()?.commands().to_vec())
    }

    /// 許可された 1 本を走らせる（D3）。
    ///
    /// **AI が渡せるのは識別子だけです。**引数で文字列がシェルへ渡る口はありません。
    /// 走るのは、人が `readonly.toml` に書いた文字列そのものです。
    ///
    /// **許可の判定はサーバーへ触る前に済ませます。**繋がっていないことより先に
    /// 「許可されていない」を返すのは、繋がった瞬間だけ何でも通る作りを
    /// テストで捕まえられるようにするためです。
    pub async fn run_readonly(&self, actor: Actor, id: &str) -> Result<Ran, EngineError> {
        let allowlist = self.allowlist()?;

        let Some(command) = allowlist.get(id) else {
            return Err(self.refuse_readonly(actor, id).await);
        };

        // 帯へは `exec` が `$ ...` を出します。**二重に出しません。**
        self.exec(actor, &command.run).await
    }

    fn allowlist(&self) -> Result<Allowlist, EngineError> {
        Allowlist::load_or_empty(&self.readonly_path())
            .map_err(|error| EngineError::Allowlist(error.to_string()))
    }

    /// 断って、**断ったことを残す**（D3 追記）。
    ///
    /// 記録できなくても帯へ出せなくても、**断るのは断ります。**
    /// 「記録できないから通す」が、ここでいちばんやってはいけない転び方です。
    async fn refuse_readonly(&self, actor: Actor, id: &str) -> EngineError {
        if let Err(error) = Refusals::at(self.readonly_refusals_path()).record(actor, id) {
            // 握り潰さない。**記録が落ちたこと自体が、許可リストの育ち方に効く。**
            self.diag.error(
                Stage::Exec,
                None,
                format!("断った記録を残せませんでした: {error}"),
                "readonly-refused.log を置くディレクトリの権限を確かめてください",
            );
        }

        let told = self
            .show(
                actor,
                &format!("run_readonly `{id}` — 許可リストに無いので断りました"),
            )
            .await;
        if let Err(error) = told {
            self.diag
                .warn(Stage::Exec, None, format!("帯へ出せませんでした: {error}"));
        }

        EngineError::NotAllowed { id: id.to_owned() }
    }

    // --- 用途別の読み取り（D3） ---------------------------------------------
    //
    // **AI はコマンドを組み立てません。**組み立てるのは `probes`、走らせるのはここ。
    // 引数を取るものは、**サーバーへ行く前に**足りているかを見ます。

    /// 空き容量。
    pub async fn disk_usage(&self, actor: Actor) -> Result<Ran, EngineError> {
        self.exec(actor, &crate::probes::disk_usage()).await
    }

    /// プロセス一覧。
    pub async fn process_list(&self, actor: Actor) -> Result<Ran, EngineError> {
        self.exec(actor, &crate::probes::process_list()).await
    }

    /// listen しているポート。
    pub async fn network_listen(&self, actor: Actor) -> Result<Ran, EngineError> {
        self.exec(actor, &crate::probes::network_listen()).await
    }

    /// サービスの状態。**名前は囲われます**（`probes`）。
    pub async fn service_status(&self, actor: Actor, name: &str) -> Result<Ran, EngineError> {
        let command = crate::probes::service_status(name)
            .map_err(|missing| EngineError::BadArgument(missing.to_string()))?;
        self.exec(actor, &command).await
    }

    /// ログの末尾。**追いかけません**（追うのは [`Engine::follow`]）。
    pub async fn read_log(&self, actor: Actor, path: &str, lines: u32) -> Result<Ran, EngineError> {
        let command = crate::probes::read_log(path, lines)
            .map_err(|missing| EngineError::BadArgument(missing.to_string()))?;
        self.exec(actor, &command).await
    }

    /// 名前で探す。**深さと件数を切ります**（切らないと返ってこない）。
    pub async fn search_names(
        &self,
        actor: Actor,
        root: &str,
        pattern: &str,
        hits: u32,
    ) -> Result<Ran, EngineError> {
        let command = crate::probes::search_names(root, pattern, hits)
            .map_err(|missing| EngineError::BadArgument(missing.to_string()))?;
        self.exec(actor, &command).await
    }

    /// 中身で探す。**バイナリは飛ばします**（混ぜると端末が壊れる）。
    pub async fn search_content(
        &self,
        actor: Actor,
        root: &str,
        pattern: &str,
        hits: u32,
    ) -> Result<Ran, EngineError> {
        let command = crate::probes::search_content(root, pattern, hits)
            .map_err(|missing| EngineError::BadArgument(missing.to_string()))?;
        self.exec(actor, &command).await
    }

    /// 何が入っていて、どの版か。**入っていないことは異常ではありません。**
    pub async fn runtime_versions(&self, actor: Actor) -> Result<Ran, EngineError> {
        self.exec(actor, &crate::probes::runtime_versions()).await
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

    // --- ダウンロード（サーバー → 手元） ------------------------------------

    /// サーバーのファイルを 1 つ手元へ落とす。
    ///
    /// **囲い（D22）はかかりません。**囲いが守るのは*サーバー*で、
    /// ここで書き換わるのは*手元*だからです。代わりに
    /// [`OnConflict`] が、**人の手元を黙って上書きしないこと**を守ります。
    ///
    /// **中身をここで全部メモリに載せます。**上げる側と同じ制限で、
    /// 分割して落とす必要のある大きさに当たったら、そのとき実測して直します（YAGNI）。
    pub async fn download_file(
        &self,
        actor: Actor,
        remote: &str,
        local: &Path,
        on_conflict: OnConflict,
    ) -> Result<u64, EngineError> {
        // **サーバーへ行く前に落とし先を確かめる。**
        // 断ったのに帯へ 1 行出た、が起きない（上げる側の `allow_write` と同じ順番）。
        check_destination(local, on_conflict)?;

        let bytes = self.read_file(actor, remote).await?;
        // ここまで来て初めて手元へ書く。**落ちてこなかったのに 0 バイトが残る、を作らない。**
        tokio::fs::write(local, &bytes)
            .await
            .map_err(|error| EngineError::Local(format!("{}: {error}", local.display())))?;
        Ok(bytes.len() as u64)
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

    /// ログインのパスワードを、その場の入力か OS ストアから引く。
    ///
    /// **人がその場で入れたものが最優先。**鍵のパスフレーズと同じ扱いです。
    /// **製品はパスワードを持ちません**（D11）。ここは通り抜けるだけです。
    fn password_for(entry: &ConnectionEntry, typed: Option<&str>) -> Option<String> {
        if let Some(typed) = typed {
            if !typed.is_empty() {
                return Some(typed.to_string());
            }
        }
        let reference = entry.keyring_password_ref.as_ref()?;
        SecretStore::new(KEYRING_SERVICE).get(reference).ok()
    }

    /// 認証のやり方を決める。**秘密はここでしか触りません**（D11 / D14）。
    fn auth_for(
        &self,
        entry: &ConnectionEntry,
        passphrase: Option<String>,
    ) -> Result<Auth, EngineError> {
        let Some(path) = entry.key_path.clone() else {
            // 鍵の指定が無い。**パスワードを預けているなら、そちらで繋ぐ。**
            //
            // 以前はここで無条件に ssh-agent へ落としており、
            // **agent に該当の鍵が無いと、そこで行き止まり**でした（実機で踏んだ）。
            // 鍵より弱くても、**この製品が置き換える相手（WinSCP / Tera Term）の
            // 利用者の多くはパスワードで繋いでいます**（PRD §0-4）。
            if let Some(password) = Self::password_for(entry, passphrase.as_deref()) {
                return Ok(Auth::Password { password });
            }
            return Ok(Auth::Agent);
        };

        // 人がその場で入れたものが最優先。無ければ OS ストアの参照名から引く。
        let stored = match (&passphrase, &entry.keyring_passphrase_ref) {
            (Some(_), _) => None,
            (None, Some(reference)) => SecretStore::new(KEYRING_SERVICE).get(reference).ok(),
            (None, None) => None,
        };

        // **中身で判定する**（D28）。拡張子は当てにならない —
        // `*.tera.ppk` の中身が OpenSSH 秘密鍵だった、が実際に在った。
        let facts = inspect_key_at(&path);
        if !facts.usable() {
            // **繋ぐ前に落ちたことを残す**（Issue #13）。
            //
            // ここには `self.diag` の呼び出しが 1 つもありませんでした。
            // しかも `connect` はこの判定を **SSH を張るより前**に行うので、
            // `reach` / `host-key` / `auth` の行も 1 本も出ません。
            // 実機で `diagnostics` が `{"events":[],"kept":0}` を返したのは、
            // **書く場所が無かった**からです。
            //
            // **鍵のパスは入れません**（PRD §8）。出すのは識別子と、読めた形式まで。
            self.diag.error(
                Stage::Auth,
                Some(&entry.id),
                format!(
                    "鍵として使えません（{} と読めました）",
                    facts.format.label()
                ),
                "秘密鍵のファイルを指してください（`.pub` は公開鍵で、認証には使えません）",
            );
            return Err(EngineError::UnusableKey {
                id: entry.id.clone(),
                format: facts.format.label().to_owned(),
            });
        }

        let secret = passphrase.or(stored);
        if secret.is_none() && facts.needs_passphrase {
            // **どの段階まで進んだかを書く**（実機の要望・Issue #13）。
            // **鍵は読めています。**止まったのはパスフレーズ待ちです。
            self.diag.error(
                Stage::Auth,
                Some(&entry.id),
                format!(
                    "鍵は読めました（{}）。パスフレーズ待ちで止まっています",
                    facts.format.label()
                ),
                "sshboard の画面で人が入れてください（AI はパスフレーズを扱いません・D14）",
            );
            return Err(EngineError::PassphraseNeeded {
                id: entry.id.clone(),
            });
        }
        // **通った分も残す。**失敗だけ残すと、
        // 「どの繋ぎ方を選んだのか」が後から読めません。
        self.diag.info(
            Stage::Auth,
            Some(&entry.id),
            format!("鍵で繋ぎます（{}）", facts.format.label()),
        );
        Ok(Auth::Key {
            path,
            passphrase: secret,
        })
    }
}

/// 「別の側が握っています」を組み立てる。**誰が握っているかを名前で返す。**
/// 記録に出す側の名前。**「人」か「AI」だけ**（PRD §8 — 宛先は入れない）。
fn who(actor: Actor) -> &'static str {
    match actor {
        Actor::Human => "人",
        Actor::Ai => "AI",
    }
}

fn held_by(holder: Actor) -> EngineError {
    EngineError::ConsoleHeldByOther {
        holder: match holder {
            Actor::Human => "人".to_string(),
            Actor::Ai => "AI".to_string(),
        },
    }
}

/// 落とし先に同じ名前があったとき、どうするか。
///
/// **既定は断る側**（`Refuse`）です。上げる側と違い、落とす側が壊すのは
/// **人の手元のファイル**で、sshboard からは元へ戻せません（product-baseline §13）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OnConflict {
    /// 既に在ったら断る。**人が「上書きする」と言うまで触らない。**
    #[default]
    Refuse,
    /// 人がその場で上書きを選んだときだけ。
    Overwrite,
}

/// 落とし先を確かめる。**サーバーへ触る前に呼びます。**
fn check_destination(local: &Path, on_conflict: OnConflict) -> Result<(), EngineError> {
    if on_conflict == OnConflict::Refuse && local.exists() {
        return Err(EngineError::Local(format!(
            "{} は既に在ります。上書きしてよいかは人が決めてください",
            local.display()
        )));
    }

    // 落とし先の階層は**勝手に作りません。**作ってしまうと、
    // 打ち間違えたパスがそのまま新しいディレクトリになり、
    // **どこへ落ちたのか分からなくなる**（上げる側の `ensure_dir` は人が明示的に押す）。
    match local.parent() {
        Some(parent) if !parent.as_os_str().is_empty() && !parent.is_dir() => {
            Err(EngineError::Local(format!(
                "{} というディレクトリがありません",
                parent.display()
            )))
        }
        _ => Ok(()),
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

/// 鍵ファイルを見て、形式とパスフレーズの要否を得る（D28）。
///
/// **中身はここを通り抜けるだけ**で、保持も記録もしません。
/// 読めないときは「判定しない」に倒します。**繋ぎに行って正直に失敗させる**方が、
/// こちらで勝手に断るより理由が分かる。
fn inspect_key_at(path: &str) -> KeyFacts {
    match std::fs::read(path) {
        Ok(bytes) => inspect_key(&bytes),
        Err(_) => KeyFacts {
            format: KeyFormat::Unknown,
            verdict: KeyVerdict::Usable,
            needs_passphrase: false,
        },
    }
}
