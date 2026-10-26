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
