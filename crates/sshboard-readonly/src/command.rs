//! 許可された 1 本。**人が書きます。**

use serde::{Deserialize, Serialize};

/// 許可リストに載った 1 本。
///
/// **`run` は AI にも見えます。**何が走るか分からないまま呼ばせる方が危ないためです。
/// 逆に言うと、**ここへ利用者名やホスト名を書くと AI に見えます**（PRD §8）。
/// 書くのは人なので、製品は止められません。`readonly.toml` の見出しで注意します。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadonlyCommand {
    /// AI が渡せる唯一の値。
    pub id: String,
    /// 実際に走る文字列。**人が書いたものが、そのまま走ります。**
    pub run: String,
    /// 何をするものか。**AI がどれを選ぶかの手がかり。**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
