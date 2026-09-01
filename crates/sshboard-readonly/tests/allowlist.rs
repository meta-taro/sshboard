//! 許可リストの読み込み（D3）。
//!
//! ここが見張るのは 2 つです。
//!
//! 1. **既定が空であること。**AI が「たぶん要るだろう」で埋めた一覧は、
//!    多すぎれば D3 の意味が消え、少なすぎれば作業が止まる。**足すのは人**（D3 追記）
//! 2. **読めない一覧を黙って空として扱わないこと。**
//!    「登録したはずなのに断られる」が一番たちが悪い

use sshboard_readonly::{Allowlist, AllowlistError, CURRENT_VERSION};

#[test]
fn a_missing_file_is_an_empty_allowlist_not_an_error() {
    // まだ 1 本も許可していないだけ。**異常ではない。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let missing = dir.path().join("readonly.toml");

    let allowlist = Allowlist::load_or_empty(&missing).expect("無いことは失敗ではない");

    assert!(allowlist.is_empty());
    assert_eq!(allowlist.commands().len(), 0);
}

#[test]
fn the_default_allowlist_ships_empty() {
    // **製品が既定で許すコマンドは 0 本。**
    // ここが 1 本でも増えていたら、誰かが推測で書いたということ（D3 追記）。
    let allowlist = Allowlist::empty();

    assert!(allowlist.is_empty());
    assert_eq!(allowlist.version, CURRENT_VERSION);
}

#[test]
fn an_unknown_version_stops_instead_of_being_treated_as_empty() {
    let error = Allowlist::parse("version = 99\n").expect_err("知らない版は止める");

    assert_eq!(error, AllowlistError::UnknownVersion { found: 99 });
}

#[test]
fn a_malformed_file_stops_instead_of_being_treated_as_empty() {
    // **空として扱うと「許可したのに断られる」になる。**握り潰さない。
    let error = Allowlist::parse("これは TOML ではありません").expect_err("読めないものは止める");

    assert!(matches!(error, AllowlistError::Malformed { .. }));
}

#[test]
fn a_listed_command_can_be_looked_up_by_id() {
    let allowlist = Allowlist::parse(
        "version = 1\n\n\
         [[command]]\n\
         id = \"uptime\"\n\
         run = \"uptime\"\n\
         description = \"稼働時間\"\n",
    )
    .expect("読める");

    let found = allowlist.get("uptime").expect("引ける");
    assert_eq!(found.run, "uptime");
    assert_eq!(found.description.as_deref(), Some("稼働時間"));
}

#[test]
fn an_unlisted_id_is_simply_absent() {
    let allowlist =
        Allowlist::parse("version = 1\n\n[[command]]\nid = \"uptime\"\nrun = \"uptime\"\n")
            .expect("読める");

    assert!(allowlist.get("rm-rf-slash").is_none());
}

#[test]
fn two_commands_with_the_same_id_stop() {
    // どちらが走るか決められない。**黙って片方を選ばない。**
    let error = Allowlist::parse(
        "version = 1\n\n\
         [[command]]\nid = \"same\"\nrun = \"uptime\"\n\n\
         [[command]]\nid = \"same\"\nrun = \"df -h\"\n",
    )
    .expect_err("重複は止める");

    assert_eq!(
        error,
        AllowlistError::DuplicateId {
            id: "same".to_string()
        }
    );
}

#[test]
fn an_empty_id_stops() {
    let error = Allowlist::parse("version = 1\n\n[[command]]\nid = \"  \"\nrun = \"uptime\"\n")
        .expect_err("空の識別子は止める");

    assert_eq!(error, AllowlistError::EmptyId);
}

#[test]
fn an_empty_command_stops() {
    // 引けるのに何も走らない項目を残さない。
    let error = Allowlist::parse("version = 1\n\n[[command]]\nid = \"nothing\"\nrun = \"\"\n")
        .expect_err("空のコマンドは止める");

    assert_eq!(
        error,
        AllowlistError::EmptyCommand {
            id: "nothing".to_string()
        }
    );
}

#[test]
fn a_command_holding_a_newline_stops() {
    // 1 項目に 2 行入ると、**人が読んだ 1 行と実際に走る中身がずれる。**
    // 許可リストは人が目で確かめられることに意味がある。
    let error =
        Allowlist::parse("version = 1\n\n[[command]]\nid = \"two\"\nrun = \"uptime\\nrm -rf /\"\n")
            .expect_err("改行を含むコマンドは止める");

    assert_eq!(
        error,
        AllowlistError::ControlCharacter {
            id: "two".to_string()
        }
    );
}

#[test]
fn the_list_keeps_the_order_the_human_wrote() {
    // 並びが毎回変わると、人が差分で確かめられない。
    let allowlist = Allowlist::parse(
        "version = 1\n\n\
         [[command]]\nid = \"b\"\nrun = \"uptime\"\n\n\
         [[command]]\nid = \"a\"\nrun = \"df -h\"\n",
    )
    .expect("読める");

    let ids: Vec<&str> = allowlist.commands().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["b", "a"]);
}
