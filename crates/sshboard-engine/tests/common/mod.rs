//! テストどうしで使う小さな道具。

use std::path::Path;

/// パスを TOML の基本文字列へ入れられる形にする。
///
/// **Windows のパスには `\` が入ります。**`C:\Users\...` をそのまま書くと
/// TOML は `\U` を unicode エスケープとして読み、
/// `invalid unicode 8-digit hex code` で**接続一覧ごと壊れます。**
/// CI（windows-latest）で実際にここが落ちました。
///
/// 製品側は `toml::to_string_pretty` を通すのでこの穴はありません。
/// **手で組んでいるテストだけの問題**ですが、直さないと
/// **Windows でだけテストが通らない**状態が残ります。
pub fn toml_string(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
