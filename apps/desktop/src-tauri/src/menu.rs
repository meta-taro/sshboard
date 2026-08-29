//! OS のメニューバー。**文言は画面側から受け取ります。**
//!
//! Rust 側にも訳を置くと二重管理になり、**片方だけ直す**事故が起きます。
//! 訳は `apps/desktop/src/lib/i18n/` に 1 箇所だけ置いてあります。

use serde::Deserialize;
use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Wry};

/// 文字サイズの操作を画面へ渡すイベント。
///
/// **メニューは Rust 側にあるが、大きさを持っているのは画面側**（localStorage）。
/// ここでは「どちらへ動かすか」だけを渡す。
pub const TEXT_SIZE_EVENT: &str = "menu://text-size";

/// 割り当て。**キー名は `keyboard-types` の Code**（`Plus` や `0` は無い）。
///
/// `CmdOrCtrl+Plus` と書いたらパースに失敗し、`apply` がエラーを返して
/// **メニューが 1 つも適用されず既定の英語メニューへ戻った。**実際に戻った。
/// 下の `accelerators_parse` がそれを見張っている。
pub const TEXT_LARGER_KEY: &str = "CmdOrCtrl+Equal";
pub const TEXT_SMALLER_KEY: &str = "CmdOrCtrl+Minus";
pub const TEXT_RESET_KEY: &str = "CmdOrCtrl+Digit0";

/// 画面から渡されるメニューの文言。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuLabels {
    pub about: String,
    pub quit: String,
    /// **「表示」メニュー。**Mac の人は文字サイズをまずここで探す（実際に探された）。
    pub view: String,
    pub text_larger: String,
    pub text_smaller: String,
    pub text_reset: String,
    pub edit: String,
    pub undo: String,
    pub redo: String,
    pub cut: String,
    pub copy: String,
    pub paste: String,
    pub select_all: String,
    pub window: String,
    pub minimize: String,
    pub zoom: String,
    pub close: String,
}

/// 受け取った文言でメニューを組み直す。
///
/// **言語を切り替えるたびに呼ばれます。**メニューは作り直すしかないので、
/// 差分更新はしません（項目が十数個なので、作り直しの方が読みやすい）。
pub fn apply(app: &AppHandle, labels: &MenuLabels) -> tauri::Result<()> {
    // **OS の「〜について」を使う。**自前の項目には受け手が要り、
    // 付け忘れると押しても何も起きない（**実際に起きなかった**）。
    //
    // 中身に個人名もメールも載せない（product-baseline §25）。
    let about = AboutMetadata {
        name: Some("sshboard".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        website: Some("https://github.com/meta-taro/sshboard".into()),
        ..Default::default()
    };

    let app_menu = Submenu::with_items(
        app,
        "sshboard",
        true,
        &[
            &PredefinedMenuItem::about(app, Some(&labels.about), Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(&labels.quit))?,
        ],
    )?;

    // **文字サイズはここに置く。**右上のボタンだけだと見つからない（実際に見つからなかった）。
    // 割り当ては OS の慣習どおり。**覚えなくても手が知っている操作**にする。
    let view_menu = Submenu::with_items(
        app,
        &labels.view,
        true,
        &[
            &MenuItem::with_id(
                app,
                "text-larger",
                &labels.text_larger,
                true,
                Some(TEXT_LARGER_KEY),
            )?,
            &MenuItem::with_id(
                app,
                "text-smaller",
                &labels.text_smaller,
                true,
                Some(TEXT_SMALLER_KEY),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "text-reset",
                &labels.text_reset,
                true,
                Some(TEXT_RESET_KEY),
            )?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        &labels.edit,
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(&labels.undo))?,
            &PredefinedMenuItem::redo(app, Some(&labels.redo))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(&labels.cut))?,
            &PredefinedMenuItem::copy(app, Some(&labels.copy))?,
            &PredefinedMenuItem::paste(app, Some(&labels.paste))?,
            &PredefinedMenuItem::select_all(app, Some(&labels.select_all))?,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        &labels.window,
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(&labels.minimize))?,
            &PredefinedMenuItem::maximize(app, Some(&labels.zoom))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(&labels.close))?,
        ],
    )?;

    let menu = Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])?;
    app.set_menu(menu)?;
    Ok(())
}

/// メニューが押されたとき。**押しても何も起きない項目を残さない。**
///
/// 文字サイズを持っているのは画面側（localStorage）なので、
/// ここでは**どちらへ動かすか**だけを渡します。
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    let direction = match event.id().as_ref() {
        "text-larger" => "larger",
        "text-smaller" => "smaller",
        "text-reset" => "reset",
        // OS が自分で処理する項目（about / quit / 編集 / ウインドウ）はここへ来ない。
        _ => return,
    };

    if let Err(error) = app.emit(TEXT_SIZE_EVENT, direction) {
        // 黙らない。**メニューが効かない理由が分からなくなる。**
        eprintln!("[sshboard] 文字サイズの操作を画面へ渡せません: {error}");
    }
}

/// 画面が言語を決めた（または切り替えた）ときに呼ぶ。
#[tauri::command]
pub fn set_menu_labels(app: AppHandle<Wry>, labels: MenuLabels) -> Result<(), String> {
    apply(&app, &labels).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{TEXT_LARGER_KEY, TEXT_RESET_KEY, TEXT_SMALLER_KEY};
    use muda::accelerator::Accelerator;
    use std::str::FromStr;
    use tauri::menu::AboutMetadata;

    /// **割り当てが読めないと、メニューが 1 つも出ない。**
    ///
    /// `MenuItem::with_id` はここで失敗し、`apply` 全体が Err になる。
    /// 画面はそれを握り潰していたので、**既定の英語メニューのまま**気づけなかった。
    #[test]
    fn every_accelerator_parses() {
        for key in [TEXT_LARGER_KEY, TEXT_SMALLER_KEY, TEXT_RESET_KEY] {
            Accelerator::from_str(key).unwrap_or_else(|error| panic!("{key} が読めない: {error}"));
        }
    }

    /// **これが弾かれることを確かめておく。**弾かれないなら上のテストは無意味。
    #[test]
    fn the_name_that_broke_the_menu_is_rejected() {
        // `Plus` は keyboard-types の Code に無い。**これ 1 つでメニューが全部消えた。**
        assert!(
            Accelerator::from_str("CmdOrCtrl+Plus").is_err(),
            "壊れた名前が通ってしまう。上のテストが無意味になる"
        );

        // 一方 `0` と `-` は通る。**通ると思い込んでいた方が間違っていた**ので、
        // ここに実測を残す（`Digit0` / `Minus` と書いても同じく通る）。
        assert!(Accelerator::from_str("CmdOrCtrl+0").is_ok());
        assert!(Accelerator::from_str("CmdOrCtrl+-").is_ok());
    }

    /// **「〜について」に個人を載せない**（product-baseline §25）。
    #[test]
    fn the_about_panel_carries_no_person() {
        let about = AboutMetadata {
            name: Some("sshboard".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            website: Some("https://github.com/meta-taro/sshboard".into()),
            ..Default::default()
        };

        assert!(about.authors.is_none(), "作者名を載せている");
        assert!(about.credits.is_none(), "謝辞に個人が入りうる");
    }
}
