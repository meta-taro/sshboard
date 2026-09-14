//! バージョンが 3 か所で揃っていることを見張る（D30）。
//!
//! **同じ番号を 3 つのファイルに書いています。**
//!
//! - `Cargo.toml`（ワークスペース）… `CARGO_PKG_VERSION` として焼き込まれる
//! - `apps/desktop/src-tauri/tauri.conf.json` … インストーラの名前と、更新の判定に使う
//! - `apps/desktop/package.json`
//!
//! **ずれても、どれもビルドは通ります。**通ったうえで、
//! 「配ったファイル名は 0.1.1 なのに、アプリが名乗るのは 0.1.0」という形で壊れます。
//! **自動更新を入れると、ここが「更新が来ない」「無限に更新し続ける」に化けます。**
//! 判定は `tauri.conf.json` の番号で行われるためです。
//!
//! なので**テストで見張ります。**人が 3 か所を手で直すのをやめて、
//! `sh tools/bump-version.sh <番号>` を使ってください。
//!
//! **このモジュールは検査だけです。**製品の動きには関わりません。

#![cfg(test)]

/// `tauri.conf.json` に書かれているバージョン。
fn tauri_conf_version() -> Option<String> {
    let raw = include_str!("../tauri.conf.json");
    json_top_level_string(raw, "version")
}

/// `package.json` に書かれているバージョン。
fn package_json_version() -> Option<String> {
    let raw = include_str!("../../package.json");
    json_top_level_string(raw, "version")
}

/// **素朴に取り出すだけ。**JSON のパーサを足すほどの用ではありません
/// （どちらのファイルも自分たちが書いており、形は決まっています）。
fn json_top_level_string(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = raw.find(&needle)? + needle.len();
    let rest = &raw[start..];
    let colon = rest.find(':')? + 1;
    let rest = &rest[colon..];
    let open = rest.find('"')? + 1;
    let rest = &rest[open..];
    let close = rest.find('"')?;
    Some(rest[..close].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_places_that_hold_the_version_agree() {
        // **ずれてもビルドは通ります。**通ったうえで、
        // 配布物の名前とアプリが名乗る番号が食い違います。
        // 自動更新を入れると「更新が来ない」「更新し続ける」に化けます。
        let cargo = env!("CARGO_PKG_VERSION");
        let tauri = tauri_conf_version().expect("tauri.conf.json から version を読めない");
        let package = package_json_version().expect("package.json から version を読めない");

        assert_eq!(
            cargo, tauri,
            "Cargo.toml と tauri.conf.json がずれています。\
             `sh tools/bump-version.sh <番号>` で 3 か所まとめて直してください"
        );
        assert_eq!(
            cargo, package,
            "Cargo.toml と package.json がずれています。\
             `sh tools/bump-version.sh <番号>` で 3 か所まとめて直してください"
        );
    }

    /// 更新の宛先。**アプリに焼き込まれるので、間違うと直せません。**
    fn updater_endpoints() -> Vec<String> {
        let raw = include_str!("../tauri.conf.json");
        let at = raw.find("\"endpoints\"").expect("endpoints が無い");
        let open = raw[at..].find('[').expect("[ が無い") + at;
        let close = raw[open..].find(']').expect("] が無い") + open;
        raw[open + 1..close]
            .split(',')
            .map(|one| one.trim().trim_matches('"').to_owned())
            .filter(|one| !one.is_empty())
            .collect()
    }

    #[test]
    fn the_new_update_address_comes_first_and_the_old_one_is_still_there() {
        // **移行の最中です**（D51 / Issue #25）。
        //
        // `updater` という版でないタグが Release の一覧に並び、
        // **実リリースが全部 prerelease だったので Latest まで奪っていました。**
        // 版のリリースへ `latest.json` を添付する形へ移します。
        //
        // **古い宛先を今すぐ消せません。**すでに入っている 0.1.5〜0.1.12 は
        // この URL を焼き込んでいて、消すと**二度と更新に気づけません。**
        //
        // **2 本とも要ります。**新しい方が先（そちらが本命）。
        let endpoints = updater_endpoints();

        assert_eq!(endpoints.len(), 2, "宛先が 2 本でない: {endpoints:?}");
        assert!(
            endpoints[0].ends_with("/releases/latest/download/latest.json"),
            "**新しい宛先が先頭でない**: {endpoints:?}"
        );
        assert!(
            endpoints[1].ends_with("/releases/download/updater/latest.json"),
            "**古い宛先が消えている**（入っている端末が更新に気づけなくなります）: {endpoints:?}"
        );
    }

    #[test]
    fn every_update_address_is_a_fixed_url() {
        // **版の番号が混ざっていたら、焼き込んだ瞬間に古くなります。**
        // D34 の条件はここです ——「アプリに焼き込む URL は変えられない」。
        let version = env!("CARGO_PKG_VERSION");

        for endpoint in updater_endpoints() {
            assert!(
                !endpoint.contains(version),
                "**宛先に版の番号が入っています**（次の版で更新が死にます）: {endpoint}"
            );
            assert!(endpoint.starts_with("https://"), "https でない: {endpoint}");
        }
    }

    #[test]
    fn the_version_looks_like_three_numbers() {
        // **prerelease をここへ書かない**（D30）。α はタグと prerelease の印で表す。
        // `0.1.0-alpha.1` と書くと 3 か所を揃えるのが途端に難しくなる。
        let version = env!("CARGO_PKG_VERSION");
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "3 つの数でない: {version}");
        for part in parts {
            assert!(
                part.chars().all(|c| c.is_ascii_digit()),
                "数でない部分がある（prerelease を書いていませんか）: {version}"
            );
        }
    }
}
