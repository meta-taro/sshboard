//! **期限付きの端末の許可**を画面から出す・外す・見る口（2026-11-16）。
//!
//! **画面にしか置きません。**MCP に出すと、AI が自分に許可を出せます ——
//! それは「AI が握るには人の許可が要る」（D42）が無くなるのと同じです。
//! 実行体の側でも、人以外からの許可は断ります（二重の歯止め）。

use std::sync::Arc;

use serde::Serialize;
use sshboard_band::Actor;
use sshboard_engine::{Engine, GrantFacts, GrantScope};
use tauri::State;

/// 1 時間の秒数。**画面からは時間の単位で受けます**（分や秒の細かい期限は要らない）。
const SECONDS_PER_HOUR: u64 = 60 * 60;

/// 画面へ渡す、いまの許可。
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GrantView {
    /// `"connection"`（この接続だけ）か `"new"`（これから増える接続だけ）。
    pub scope: &'static str,
    /// `scope` が `"connection"` のときの接続の識別子。
    pub connection: Option<String>,
    /// 残りの秒数。
    pub remaining_secs: u64,
}

impl From<GrantFacts> for GrantView {
    fn from(facts: GrantFacts) -> Self {
        let (scope, connection) = match facts.scope {
            GrantScope::Connection(id) => ("connection", Some(id)),
            GrantScope::NewConnections => ("new", None),
        };
        Self {
            scope,
            connection,
            remaining_secs: facts.remaining.as_secs(),
        }
    }
}

/// 画面の言い方を、実行体の範囲へ。**知らない言い方は断ります**（推し量って広い方へ倒さない）。
fn scope_from(scope: &str, connection: Option<String>) -> Result<GrantScope, String> {
    match (scope, connection) {
        ("new", _) => Ok(GrantScope::NewConnections),
        ("connection", Some(id)) if !id.trim().is_empty() => Ok(GrantScope::Connection(id)),
        ("connection", _) => Err("どの接続への許可か、接続を選んでください".to_owned()),
        (other, _) => Err(format!("許可の範囲 `{other}` は知りません")),
    }
}

/// 許可を出す。**押しているのは人**です（画面の口なので）。
#[tauri::command]
pub async fn console_grant(
    scope: String,
    connection: Option<String>,
    hours: u64,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<GrantView>, String> {
    let scope = scope_from(&scope, connection)?;
    if hours == 0 {
        return Err("期限は 1 時間以上にしてください".to_owned());
    }
    let for_how_long = std::time::Duration::from_secs(hours.saturating_mul(SECONDS_PER_HOUR));
    engine
        .console_grant(Actor::Human, scope, for_how_long)
        .await
        .map_err(|error| error.to_string())?;
    Ok(engine.console_grant_facts().await.map(GrantView::from))
}

/// 許可を外す。
#[tauri::command]
pub async fn console_grant_revoke(engine: State<'_, Arc<Engine>>) -> Result<(), String> {
    engine.console_grant_revoke(Actor::Human).await;
    Ok(())
}

/// いまの許可。**期限を過ぎていれば `None`。**
#[tauri::command]
pub async fn console_grant_state(
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<GrantView>, String> {
    Ok(engine.console_grant_facts().await.map(GrantView::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_scope_is_refused_rather_than_widened() {
        assert!(scope_from("all", None).is_err());
    }

    #[test]
    fn a_connection_scope_needs_a_connection() {
        assert!(scope_from("connection", None).is_err());
        assert!(scope_from("connection", Some("  ".into())).is_err());
        assert_eq!(
            scope_from("connection", Some("vm".into())),
            Ok(GrantScope::Connection("vm".into()))
        );
    }

    #[test]
    fn the_view_carries_scope_connection_and_seconds_left() {
        let view = GrantView::from(GrantFacts {
            scope: GrantScope::Connection("vm".into()),
            remaining: std::time::Duration::from_secs(90),
        });
        assert_eq!(view.scope, "connection");
        assert_eq!(view.connection.as_deref(), Some("vm"));
        assert_eq!(view.remaining_secs, 90);
    }
}
