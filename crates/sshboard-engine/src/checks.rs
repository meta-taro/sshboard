//! **点検表を回す**（D72）。人が `checks.toml` に書いた手順を、書いた順に走らせて判定します。
//!
//! 各手順は `run_readonly` と**同じ道**を通ります —— 帯に出てから走り、
//! 許可リストに無ければ断られ、断った記録も残ります。点検のための別の口は作りません。

use std::collections::HashMap;

use serde::Serialize;
use sshboard_band::Actor;
use sshboard_readonly::{Check, Checks, Judged, StepOutcome, Verdict};
use tokio::sync::{watch, Mutex};

use crate::engine::Engine;
use crate::error::EngineError;

/// 手順 1 つの結果。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepReport {
    /// `readonly.toml` の id。
    pub readonly: String,
    /// 人に見せる名前（無ければ id）。
    pub title: String,
    pub judged: Judged,
    /// **前回の同じ手順**（同じ点検・同じ接続）。初めてなら無い。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<Judged>,
}

/// 点検 1 回ぶん。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckReport {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// どの接続で回したか（識別子だけ。ホスト名は持たない）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    pub steps: Vec<StepReport>,
    pub good: usize,
    pub bad: usize,
    pub unknown: usize,
}

/// **前回の結果と、AI の案。メモリだけ**（アプリを終えると消える・D72）。
pub(crate) struct CheckMemory {
    /// 点検 id × 接続ごとの最後の結果（画面に出すもの）。
    last: Mutex<HashMap<(String, String), CheckReport>>,
    /// **手順ごとの、最後に実際に測れた値**（前回の値はここから）。
    ///
    /// 測れなかった回（繋がっていない・許可外）で上書きすると、
    /// 前回の値が空になって比べられなくなります（コードレビュー M1）。
    measured: Mutex<HashMap<StepKey, Judged>>,
    /// **AI が出した点検の案**（まだ保存されていない）。1 枠。
    proposal: watch::Sender<Option<Check>>,
}

impl Default for CheckMemory {
    fn default() -> Self {
        Self {
            last: Mutex::default(),
            measured: Mutex::default(),
            proposal: watch::channel(None).0,
        }
    }
}

/// 点検 id・接続・手順の位置・手順の id。**位置か id が変われば別の手順**です。
type StepKey = (String, String, usize, String);

impl CheckMemory {
    async fn measured(&self, key: &StepKey) -> Option<Judged> {
        self.measured.lock().await.get(key).cloned()
    }

    async fn remember_measured(&self, key: StepKey, judged: Judged) {
        self.measured.lock().await.insert(key, judged);
    }

    async fn previous(&self, check: &str, connection: &str) -> Option<CheckReport> {
        let key = (check.to_owned(), connection.to_owned());
        self.last.lock().await.get(&key).cloned()
    }

    async fn remember(&self, report: &CheckReport) {
        let key = (
            report.id.clone(),
            report.connection.clone().unwrap_or_default(),
        );
        self.last.lock().await.insert(key, report.clone());
    }
}

impl Engine {
    /// 点検表の置き場所。**接続一覧と同じディレクトリ。**
    pub fn checks_path(&self) -> std::path::PathBuf {
        self.readonly_path().with_file_name("checks.toml")
    }

    /// 人が書いた点検全部。**既定は空。**
    pub fn checks(&self) -> Result<Checks, EngineError> {
        Checks::load_or_empty(&self.checks_path())
            .map_err(|error| EngineError::Checks(error.to_string()))
    }

    /// **いまの宛先で最後に回した結果**（人でも AI でも）。画面が読みます。
    pub async fn last_check(&self, id: &str) -> Option<CheckReport> {
        let connection = self.active().await.map(|opened| opened.id);
        self.check_memory()
            .previous(id, connection.as_deref().unwrap_or_default())
            .await
    }

    /// 点検を 1 つ回す。**悪しや測れずが出ても、最後の手順まで回します。**
    pub async fn run_check(&self, actor: Actor, id: &str) -> Result<CheckReport, EngineError> {
        let checks = self.checks()?;
        let Some(check) = checks.get(id) else {
            return Err(EngineError::UnknownCheck {
                id: id.to_owned(),
                have: checks
                    .checks()
                    .iter()
                    .map(|check| check.id.clone())
                    .collect(),
                path: self.checks_path().display().to_string(),
            });
        };

        // **名指しがあればその接続**（AI の道具・2026-11-11）。無ければ画面の宛先。
        let connection = self.target_id().await;
        let on = connection.clone().unwrap_or_default();

        let mut steps = Vec::with_capacity(check.steps.len());
        let mut moved: Option<String> = None;
        for (index, step) in check.steps.iter().enumerate() {
            // **途中で宛先が変わったら、残りは測らない**（コードレビュー M2）。
            if moved.is_none() {
                let now = self.target_id().await;
                moved = moved_since(connection.as_deref(), now.as_deref());
            }
            let outcome = match &moved {
                Some(why) => StepOutcome::DidNotRun { why: why.clone() },
                None => match self.run_readonly(actor, &step.readonly).await {
                    Ok(ran) => StepOutcome::Ran {
                        out: ran.out,
                        status: ran.status,
                    },
                    Err(error) => StepOutcome::DidNotRun {
                        why: error.to_string(),
                    },
                },
            };
            let key = (id.to_owned(), on.clone(), index, step.readonly.clone());
            let previous = self.check_memory().measured(&key).await;
            let judged = step.judge(&outcome);
            if matches!(outcome, StepOutcome::Ran { .. }) {
                self.check_memory()
                    .remember_measured(key, judged.clone())
                    .await;
            }
            steps.push(StepReport {
                readonly: step.readonly.clone(),
                title: step.title().to_owned(),
                judged,
                previous,
            });
        }

        let report = summarise(check, connection, steps);
        self.check_memory().remember(&report).await;
        Ok(report)
    }
}

impl Engine {
    /// いま人の答えを待っている点検の案。
    pub fn check_proposal(&self) -> Option<Check> {
        self.check_memory().proposal.borrow().clone()
    }

    pub fn subscribe_check_proposal(&self) -> watch::Receiver<Option<Check>> {
        self.check_memory().proposal.subscribe()
    }

    /// **AI が点検の案を出す。**書き込みはしません —— 保存するのは画面の人です（D22 / D27）。
    ///
    /// 人に見せる前に検品します。許可リストに無い手順・既に在る点検の上書き・
    /// 別の案が答えを待っている、はここで断ります（見せてから断ると「押したのに入らない」になる）。
    pub async fn propose_check(&self, actor: Actor, check: Check) -> Result<(), EngineError> {
        within_limits(&check)?;
        let parsed = Checks::parse(&format!("version = 1\n{}", check.to_toml_block()))
            .map_err(|error| EngineError::BadArgument(error.to_string()))?;
        if parsed.get(&check.id) != Some(&check) {
            return Err(EngineError::BadArgument(format!(
                "点検 `{}` の案を表の形に直せませんでした",
                check.id
            )));
        }
        self.only_listed_steps(&check)?;
        if self.checks()?.get(&check.id).is_some() {
            return Err(EngineError::BadArgument(format!(
                "点検 `{}` は既に在ります。**上書きしません** —— 直すなら人が {} を開きます",
                check.id,
                self.checks_path().display()
            )));
        }
        let waiting = self.check_proposal().map(|held| held.id);
        if let Some(waiting) = waiting.filter(|held| *held != check.id) {
            return Err(EngineError::ProposalPending { id: waiting });
        }

        let steps: Vec<&str> = check
            .steps
            .iter()
            .map(|step| step.readonly.as_str())
            .collect();
        let shown = format!(
            "propose_check `{}` — **保存してよいか聞いています**: {}",
            check.id,
            steps.join(" → ")
        );
        if let Err(error) = self.show(actor, &shown).await {
            self.diag_warn(format!("帯へ出せませんでした: {error}"));
        }
        // **受け手が居なくても値を入れる**（operation の提案で 1 度消えた・`send_replace`）。
        self.check_memory().proposal.send_replace(Some(check));
        Ok(())
    }

    /// **人が案を保存する。**`checks.toml` へ書き足します（人の書いたものは消しません）。
    ///
    /// **ここを呼べるのは画面だけ**です。AI がこのファイルを書く道はありません。
    ///
    /// **画面が見せていた案を渡します。**いま持っている案と違えば保存しません ——
    /// 見せたあとで AI が差し替えた案を、人が見ないまま保存しない（セキュリティレビュー MEDIUM-1）。
    pub fn save_check_proposal(&self, shown: &Check) -> Result<Check, EngineError> {
        let Some(check) = self.check_proposal() else {
            return Err(EngineError::BadArgument(
                "保存する点検の案がありません".to_owned(),
            ));
        };
        if check != *shown {
            return Err(EngineError::BadArgument(format!(
                "点検 `{}` の案は、画面に出たあとで差し替わりました。いまの案を見直してから保存してください",
                check.id
            )));
        }
        let path = self.checks_path();
        let held = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                "version = 1\n".to_owned()
            }
            Err(error) => return Err(EngineError::Checks(error.to_string())),
        };
        let text = format!("{}\n{}", held.trim_end(), check.to_toml_block());
        // **書いたものが読めることを、書く前に確かめる**（読めない表は 1 本も回らない）。
        let parsed =
            Checks::parse(&text).map_err(|error| EngineError::BadArgument(error.to_string()))?;
        if parsed.get(&check.id).is_none() {
            return Err(EngineError::BadArgument(format!(
                "点検 `{}` を書き足せませんでした",
                check.id
            )));
        }
        write_whole(&path, &text)?;
        self.check_memory().proposal.send_replace(None);
        Ok(check)
    }

    /// **人が案を断る。**何も書きません。
    pub fn dismiss_check_proposal(&self) {
        self.check_memory().proposal.send_replace(None);
    }

    /// 手順が指す id が、すべて許可リストに在るか。**無いものは在るものを添えて断る。**
    fn only_listed_steps(&self, check: &Check) -> Result<(), EngineError> {
        let listed = self.readonly_commands()?;
        let missing: Vec<&str> = check
            .steps
            .iter()
            .map(|step| step.readonly.as_str())
            .filter(|id| !listed.iter().any(|command| command.id == *id))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let have: Vec<&str> = listed.iter().map(|command| command.id.as_str()).collect();
        Err(EngineError::BadArgument(format!(
            "許可リストに無い手順があります: {}（在るもの: {}）。手順は list_readonly_commands の id だけを指せます",
            missing.join(" , "),
            have.join(" , ")
        )))
    }
}

/// **一時ファイルに書いて置き換える**（コードレビュー L1）。
///
/// そのまま書くと、書いている途中で落ちたとき、人が育てた表が空になります。
fn write_whole(path: &std::path::Path, text: &str) -> Result<(), EngineError> {
    let fail = |error: std::io::Error| {
        EngineError::Checks(format!("{} へ書けません: {error}", path.display()))
    };
    let partial = path.with_extension("toml.partial");
    std::fs::write(&partial, text).map_err(fail)?;
    std::fs::rename(&partial, path).map_err(|error| {
        // 置き換えられなかった書きかけは残さない。**消せなくても、元の表は無傷です。**
        let _ = std::fs::remove_file(&partial);
        fail(error)
    })
}

/// 案の手順の上限。**画面を埋める案で、大事な手順を見落とさせない**（セキュリティレビュー LOW-1）。
const MAX_PROPOSED_STEPS: usize = 20;
/// 案の 1 つの欄の文字数の上限。
const MAX_PROPOSED_CHARS: usize = 200;

/// **AI の案だけに掛ける上限。**人が自分で書く表は縛りません。
fn within_limits(check: &Check) -> Result<(), EngineError> {
    if check.steps.len() > MAX_PROPOSED_STEPS {
        return Err(EngineError::BadArgument(format!(
            "手順は {MAX_PROPOSED_STEPS} 個までです（{} 個ありました）。分けて出してください",
            check.steps.len()
        )));
    }
    let texts = [Some(check.id.as_str()), check.description.as_deref()]
        .into_iter()
        .chain(check.steps.iter().flat_map(|step| {
            [
                Some(step.readonly.as_str()),
                step.label.as_deref(),
                step.expect.contains.as_deref(),
                step.expect.absent.as_deref(),
            ]
        }));
    if texts
        .flatten()
        .any(|text| text.chars().count() > MAX_PROPOSED_CHARS)
    {
        return Err(EngineError::BadArgument(format!(
            "案の欄は {MAX_PROPOSED_CHARS} 文字までです。短くして出してください"
        )));
    }
    Ok(())
}

/// **回し始めた宛先から、いまの宛先が変わったか。**変わっていれば理由を返します。
///
/// 前半は A・後半は B の値が A の記録に混ざるのを防ぐため、変わったら残りは測りません。
pub fn moved_since(started: Option<&str>, now: Option<&str>) -> Option<String> {
    if started == now {
        return None;
    }
    let name = |id: Option<&str>| id.unwrap_or("（繋がっていない）").to_owned();
    Some(format!(
        "回している途中で宛先が変わりました（{} → {}）。もう一度回してください",
        name(started),
        name(now)
    ))
}

fn summarise(check: &Check, connection: Option<String>, steps: Vec<StepReport>) -> CheckReport {
    let count = |wanted: fn(&Verdict) -> bool| {
        steps
            .iter()
            .filter(|step| wanted(&step.judged.verdict))
            .count()
    };
    let good = count(|verdict| matches!(verdict, Verdict::Good));
    let bad = count(|verdict| matches!(verdict, Verdict::Bad { .. }));
    let unknown = count(|verdict| matches!(verdict, Verdict::Unknown { .. }));
    CheckReport {
        id: check.id.clone(),
        description: check.description.clone(),
        connection,
        steps,
        good,
        bad,
        unknown,
    }
}
