//! **起動時に回すものが、本当に回されているか**（2026-10-26）。
//!
//! 実行体に片付けの口を足しても、アプリが回さなければ実機では何も起きません ——
//! 試験は通り、画面だけが古いまま、という形になります。

const LIB: &str = include_str!("../src/lib.rs");

#[test]
fn the_app_starts_the_console_reaper() {
    // シェルが `exit` で終わった端末を閉じる片付け。回さないと「ログアウト」のまま残る。
    assert!(
        LIB.contains(".run_console_reaper()"),
        "**シェルが終わった端末を閉じる片付けを、アプリが回していません**"
    );
}

#[test]
fn the_app_exposes_the_standing_allowance_commands() {
    // 「外すまで許可」の口。**登録し忘れると、画面から呼んでも届きません。**
    for command in [
        "session_cmd::operation_answer_standing",
        "session_cmd::standing_list",
        "session_cmd::standing_revoke",
        "session_cmd::standing_revoke_all",
    ] {
        assert!(LIB.contains(command), "{command} が登録されていません");
    }
}

#[test]
fn the_app_exposes_the_check_commands() {
    // 点検表（D72）。**登録し忘れると、画面の［回す］が届きません。**
    for command in ["check_cmd::checks_overview", "check_cmd::check_run"] {
        assert!(LIB.contains(command), "{command} が登録されていません");
    }
}

#[test]
fn the_app_exposes_the_check_proposal_and_pushes_it_to_the_screen() {
    // AI の点検の案（D72 の次の段）。**押し出さないと人は気づかず、登録しないと保存の口が届かない。**
    for wired in [
        "check_cmd::check_proposal",
        "check_cmd::check_proposal_save",
        "check_cmd::check_proposal_dismiss",
        "check_cmd::spawn_check_proposal_bridge(",
    ] {
        assert!(LIB.contains(wired), "{wired} が繋がっていません");
    }
}
