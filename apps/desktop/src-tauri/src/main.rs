// Windows で release ビルドしたときにコンソール窓を出さない。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sshboard_desktop_lib::run()
}
