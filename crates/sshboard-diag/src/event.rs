//! 記録の 1 件。**そのまま画面にも MCP の応答にも出ます。**

use serde::Serialize;

/// どれくらい深刻か。**「困っている」と「起きたことの報告」を混ぜない。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// 起きたことの報告。
    Info,
    /// 通ったが、気に留めてほしい。
    Warn,
    /// 進めなかった。**ここには必ず `hint` を付ける。**
    Error,
}

/// どの段階か。**「繋がらない」を段階に割って、どこで止まったかを言えるようにする。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    /// TCP で相手に届くか。
    Reach,
    /// ホスト鍵を信用してよいか。
    HostKey,
    /// 認証。**ここで一番詰まる。**
    Auth,
    /// 繋がったあとのファイル操作。
    Sftp,
    /// 繋がったあとのコマンド。
    Exec,
    /// MCP からの呼び出し。
    Mcp,
    /// 接続一覧の読み書き。
    Registry,
}

/// 記録の 1 件。
///
/// **接続先を入れません**（PRD §8）。入れてよいのは接続の識別子までです。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub seq: u64,
    /// アプリを起動してからの経過ミリ秒。
    ///
    /// **時刻を持ちません。**記録を貼ったときに、その人がいつ何をしていたかが
    /// 分かってしまう必要はなく、**知りたいのは前後関係と所要時間**だからです。
    pub at_ms: u64,
    pub level: Level,
    pub stage: Stage,
    /// どの接続の話か。**識別子だけ**（`web-prod`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    pub message: String,
    /// **次に何をすればよいか。**`Error` には必ず付けます。
    ///
    /// 「駄目でした」で終わらせない（product-baseline §17）。
    /// これが無いと、人も AI も手が出ません。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Event {
    /// 人が 1 行で読める形。**画面と端末で同じ形にする。**
    pub fn render(&self) -> String {
        let seconds = self.at_ms as f64 / 1000.0;
        let level = match self.level {
            Level::Info => "情報",
            Level::Warn => "注意",
            Level::Error => "失敗",
        };
        let stage = match self.stage {
            Stage::Reach => "到達",
            Stage::HostKey => "ホスト鍵",
            Stage::Auth => "認証",
            Stage::Sftp => "ファイル",
            Stage::Exec => "コマンド",
            Stage::Mcp => "MCP",
            Stage::Registry => "接続一覧",
        };
        let who = self
            .connection
            .as_deref()
            .map(|id| format!(" [{id}]"))
            .unwrap_or_default();
        let hint = self
            .hint
            .as_deref()
            .map(|hint| format!("\n           → {hint}"))
            .unwrap_or_default();

        format!("{seconds:7.3}s {level} {stage}{who} {}{hint}", self.message)
    }
}
