//! **点検表を画面へ**（D72）。人が書いた点検を並べ、［回す］で回し、最後の結果を見せます。
//!
//! 回す道は AI と同じ `Engine::run_check` です。**AI が回した結果も、ここで見えます**
//! （前回の値は点検 id × 接続ごとに実行体が持っています）。

use std::sync::Arc;

use sshboard_band::Actor;
use sshboard_engine::{CheckReport, Engine};
use tauri::State;

/// 点検 1 つの見出しと手順の名前。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckLine {
    pub id: String,
    pub description: Option<String>,
    pub steps: Vec<String>,
    /// いまの宛先で最後に回した結果（人でも AI でも）。
    pub last: Option<CheckReport>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksOverview {
    /// 人が書くファイル。**絶対パスで言います**（探させない）。
    pub path: String,
    pub checks: Vec<CheckLine>,
}

#[tauri::command]
pub async fn checks_overview(engine: State<'_, Arc<Engine>>) -> Result<ChecksOverview, String> {
    let checks = engine.checks().map_err(|error| error.to_string())?;
    let mut lines = Vec::with_capacity(checks.checks().len());
    for check in checks.checks() {
        lines.push(CheckLine {
            id: check.id.clone(),
            description: check.description.clone(),
            steps: check
                .steps
                .iter()
                .map(|step| step.title().to_owned())
                .collect(),
            last: engine.last_check(&check.id).await,
        });
    }
    Ok(ChecksOverview {
        path: engine.checks_path().display().to_string(),
        checks: lines,
    })
}

#[tauri::command]
pub async fn check_run(engine: State<'_, Arc<Engine>>, id: String) -> Result<CheckReport, String> {
    engine
        .run_check(Actor::Human, &id)
        .await
        .map_err(|error| error.to_string())
}
