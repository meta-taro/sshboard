//! **点検表を回す**（D72）。人が `checks.toml` に書いた手順を、書いた順に走らせて判定します。
//!
//! 各手順は `run_readonly` と**同じ道**を通ります —— 帯に出てから走り、
//! 許可リストに無ければ断られ、断った記録も残ります。点検のための別の口は作りません。

use std::collections::HashMap;

use serde::Serialize;
use sshboard_band::Actor;
use sshboard_readonly::{Check, Checks, Judged, StepOutcome, Verdict};
use tokio::sync::Mutex;

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

/// **前回の結果。メモリだけ**（アプリを終えると消える・D72）。点検 id × 接続ごと。
#[derive(Default)]
pub(crate) struct CheckMemory {
    last: Mutex<HashMap<(String, String), CheckReport>>,
}

impl CheckMemory {
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

        let connection = self.active().await.map(|opened| opened.id);
        let before = self
            .check_memory()
            .previous(id, connection.as_deref().unwrap_or_default())
            .await;

        let mut steps = Vec::with_capacity(check.steps.len());
        for (index, step) in check.steps.iter().enumerate() {
            let outcome = match self.run_readonly(actor, &step.readonly).await {
                Ok(ran) => StepOutcome::Ran {
                    out: ran.out,
                    status: ran.status,
                },
                Err(error) => StepOutcome::DidNotRun {
                    why: error.to_string(),
                },
            };
            let previous = before
                .as_ref()
                .and_then(|report| report.steps.get(index))
                .filter(|earlier| earlier.readonly == step.readonly)
                .map(|earlier| earlier.judged.clone());
            steps.push(StepReport {
                readonly: step.readonly.clone(),
                title: step.title().to_owned(),
                judged: step.judge(&outcome),
                previous,
            });
        }

        let report = summarise(check, connection, steps);
        self.check_memory().remember(&report).await;
        Ok(report)
    }
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
