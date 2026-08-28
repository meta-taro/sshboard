//! **AI が書いてよい場所の囲い**（D22）。
//!
//! Phase 1 を読み取り専用にした D2 を覆すとき、覆し方が「使わない約束」だと
//! 何も守れません（PRD §3「約束は手順書であって、ゲートではない」）。
//! だから **書ける場所そのものを列挙**し、そこから外れた書き込みは**呼びようがない**形にします。
//!
//! - **既定は `Denied`。**設定していない接続では、AI は 1 バイトも書けません。
//! - 囲いがかかるのは **AI の口だけ**です。人（GUI）は従来どおり制限しません（PRD §3）。

/// AI が書いてよい範囲。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WriteScope {
    /// **既定。**書き込みを一切許さない。
    #[default]
    Denied,
    /// 列挙した絶対パスの**下だけ**許す。
    Under(Vec<String>),
}

/// 断った理由。**「駄目でした」で終わらせない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// この接続には書き込み許可ディレクトリが 1 つも設定されていない。
    NothingAllowed,
    /// 相対パス。囲いは絶対パスでしか判定できない。
    NotAbsolute,
    /// `..` を含む。囲いの外へ出る余地を残さない。
    HasParentComponent,
    /// 絶対パスだが、どの許可ディレクトリの下でもない。
    OutsideScope,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NothingAllowed => write!(
                f,
                "この接続には書き込み許可ディレクトリが設定されていません。\
                 sshboard の画面で、書いてよいディレクトリを人が指定してください"
            ),
            Refusal::NotAbsolute => write!(f, "絶対パスで指定してください"),
            Refusal::HasParentComponent => write!(f, "パスに `..` は使えません"),
            Refusal::OutsideScope => write!(
                f,
                "書き込み許可ディレクトリの外です。\
                 そこへ書く必要があるなら、人に許可ディレクトリを足してもらってください"
            ),
        }
    }
}

impl WriteScope {
    /// 許可ディレクトリの一覧から作る。**絶対パスでないものは黙って落とさず、弾く。**
    pub fn under<I, S>(roots: I) -> Result<Self, Refusal>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut kept = Vec::new();
        for root in roots {
            let root = normalize(root.as_ref());
            check_shape(&root)?;
            kept.push(root);
        }
        if kept.is_empty() {
            return Ok(WriteScope::Denied);
        }
        Ok(WriteScope::Under(kept))
    }

    /// 許可ディレクトリの一覧。**人へ見せるため。**
    pub fn roots(&self) -> &[String] {
        match self {
            WriteScope::Denied => &[],
            WriteScope::Under(roots) => roots,
        }
    }

    /// **ディレクトリ**として触ってよいか。
    ///
    /// 許可ディレクトリ**そのもの**も含む。無いなら作れないと、囲いの中で何もできない。
    pub fn permits_dir(&self, path: &str) -> Result<(), Refusal> {
        let WriteScope::Under(roots) = self else {
            return Err(Refusal::NothingAllowed);
        };
        let path = normalize(path);
        check_shape(&path)?;
        if roots.contains(&path) {
            return Ok(());
        }
        self.permits(&path)
    }

    /// この宛先へ**ファイルとして**書いてよいか。
    pub fn permits(&self, path: &str) -> Result<(), Refusal> {
        let WriteScope::Under(roots) = self else {
            return Err(Refusal::NothingAllowed);
        };

        let path = normalize(path);
        check_shape(&path)?;

        // 許可ディレクトリ **そのもの** は宛先にできない（ディレクトリを上書きする指定は無意味）。
        // 一致だけを弾いて、下は通す。
        for root in roots {
            let prefix = if root == "/" {
                "/".to_string()
            } else {
                format!("{root}/")
            };
            if path != *root && path.starts_with(&prefix) {
                return Ok(());
            }
        }
        Err(Refusal::OutsideScope)
    }
}

/// 末尾の `/` を落とし、連続した `/` を 1 つへ潰す。**`/` 自身は残す。**
fn normalize(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut previous_was_slash = false;
    for ch in path.chars() {
        if ch == '/' {
            if previous_was_slash {
                continue;
            }
            previous_was_slash = true;
        } else {
            previous_was_slash = false;
        }
        out.push(ch);
    }
    while out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    out
}

/// 形だけを見る。**中身の存在は見ない**（存在確認はサーバー側の仕事）。
fn check_shape(path: &str) -> Result<(), Refusal> {
    if !path.starts_with('/') {
        return Err(Refusal::NotAbsolute);
    }
    if path.split('/').any(|component| component == "..") {
        return Err(Refusal::HasParentComponent);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_scope_refuses_everything() {
        // **設定していない接続では、AI は 1 バイトも書けない。**ここが崩れると囲いが意味を失う。
        let scope = WriteScope::default();

        assert_eq!(scope.permits("/srv/app/x"), Err(Refusal::NothingAllowed));
        assert_eq!(scope.roots(), &[] as &[String]);
    }

    #[test]
    fn a_path_under_an_allowed_root_is_permitted() {
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(scope.permits("/srv/app/release/app.tar.gz"), Ok(()));
    }

    #[test]
    fn a_sibling_directory_sharing_a_name_prefix_is_refused() {
        // `/srv/app` が `/srv/apple` を通してしまうのが、素朴な前方一致の壊れ方。
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(scope.permits("/srv/apple/x"), Err(Refusal::OutsideScope));
    }

    #[test]
    fn the_allowed_root_itself_is_not_a_valid_destination() {
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(scope.permits("/srv/app"), Err(Refusal::OutsideScope));
        assert_eq!(scope.permits("/srv/app/"), Err(Refusal::OutsideScope));
    }

    #[test]
    fn a_parent_component_is_refused_even_when_the_result_would_stay_inside() {
        // `..` を許すと、囲いの判定を文字列に頼れなくなる。**形の時点で弾く。**
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(
            scope.permits("/srv/app/../app/x"),
            Err(Refusal::HasParentComponent)
        );
        assert_eq!(
            scope.permits("/srv/app/../../etc/passwd"),
            Err(Refusal::HasParentComponent)
        );
    }

    #[test]
    fn a_relative_path_is_refused() {
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(scope.permits("srv/app/x"), Err(Refusal::NotAbsolute));
        assert_eq!(scope.permits("./x"), Err(Refusal::NotAbsolute));
    }

    #[test]
    fn a_relative_root_is_refused_rather_than_silently_dropped() {
        // 黙って落とすと「1 件設定したのに効かない」という気づけない壊れ方になる。
        assert_eq!(WriteScope::under(["srv/app"]), Err(Refusal::NotAbsolute));
        assert_eq!(
            WriteScope::under(["/srv/app", "../etc"]),
            Err(Refusal::NotAbsolute)
        );
    }

    #[test]
    fn an_empty_root_list_becomes_denied_not_allow_everything() {
        // 空を「制限なし」と読むのが、この種の囲いの典型的な事故。
        assert_eq!(
            WriteScope::under(Vec::<String>::new()),
            Ok(WriteScope::Denied)
        );
    }

    #[test]
    fn trailing_and_duplicated_slashes_do_not_change_the_decision() {
        let scope = WriteScope::under(["/srv/app/"]).expect("作れない");

        assert_eq!(scope.roots(), &["/srv/app".to_string()]);
        assert_eq!(scope.permits("/srv//app///x"), Ok(()));
    }

    #[test]
    fn root_as_an_allowed_directory_permits_anything_below_it() {
        // `/` を許可するのは危ないが、**人が明示的にそう設定したなら通す。**
        // 危険は「気づかず通る」ことであって、「人が選んだ」ことではない。
        let scope = WriteScope::under(["/"]).expect("作れない");

        assert_eq!(scope.permits("/etc/passwd"), Ok(()));
        assert_eq!(scope.permits("/"), Err(Refusal::OutsideScope));
    }

    #[test]
    fn the_allowed_root_is_valid_as_a_directory_though_not_as_a_file() {
        // 許可ディレクトリ自身を作れないと、囲いの中で何もできない。
        // 一方でそこへ**ファイルを**書く指定は無意味なので、そちらは弾いたまま。
        let scope = WriteScope::under(["/srv/app"]).expect("作れない");

        assert_eq!(scope.permits_dir("/srv/app"), Ok(()));
        assert_eq!(scope.permits_dir("/srv/app/release"), Ok(()));
        assert_eq!(scope.permits("/srv/app"), Err(Refusal::OutsideScope));
        assert_eq!(scope.permits_dir("/srv"), Err(Refusal::OutsideScope));
    }

    #[test]
    fn the_default_scope_refuses_directories_too() {
        assert_eq!(
            WriteScope::default().permits_dir("/srv/app"),
            Err(Refusal::NothingAllowed)
        );
    }

    #[test]
    fn several_roots_are_all_honoured() {
        let scope = WriteScope::under(["/srv/app", "/var/www/html"]).expect("作れない");

        assert_eq!(scope.permits("/var/www/html/index.php"), Ok(()));
        assert_eq!(scope.permits("/srv/app/a"), Ok(()));
        assert_eq!(scope.permits("/tmp/a"), Err(Refusal::OutsideScope));
    }
}
