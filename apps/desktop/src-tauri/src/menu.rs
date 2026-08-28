//! OS のメニューバー。**文言は画面側から受け取ります。**
//!
//! Rust 側にも訳を置くと二重管理になり、**片方だけ直す**事故が起きます。
//! 訳は `apps/desktop/src/lib/i18n/` に 1 箇所だけ置いてあります。

use serde::Deserialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

/// 画面から渡されるメニューの文言。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuLabels {
    pub about: String,
    pub quit: String,
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
    let app_menu = Submenu::with_items(
        app,
        "sshboard",
        true,
        &[
            &MenuItem::with_id(app, "about", &labels.about, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(&labels.quit))?,
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

    let menu = Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])?;
    app.set_menu(menu)?;
    Ok(())
}

/// 画面が言語を決めた（または切り替えた）ときに呼ぶ。
#[tauri::command]
pub fn set_menu_labels(app: AppHandle<Wry>, labels: MenuLabels) -> Result<(), String> {
    apply(&app, &labels).map_err(|error| error.to_string())
}
