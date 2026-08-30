//! 画面を撮る（D26）。**伏せ終わってから撮ります。**
//!
//! 撮ってから画像を加工する形にすると、**伏せていない元画像が一瞬でも存在します。**
//! ここでは先に画面へ「伏せろ」と言い、**画面が伏せ終わったと返してから**撮ります。
//!
//! 撮ったものは**ファイルに書きません。**MCP の応答として渡すだけで、
//! リポジトリにも作業ディレクトリにも残りません（D26）。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use sshboard_mcp::{WindowCapture, WindowShot};
use tauri::{AppHandle, Emitter, Listener, Manager};
use tokio::sync::oneshot;

/// 画面側と取り決めたイベント名。**片方だけ変えると伏せずに撮ります。**
const REDACT_EVENT: &str = "capture://redact";
const REDACT_READY_EVENT: &str = "capture://ready";

/// 伏せ終わるのを待つ上限。
///
/// **待てなかったら撮りません。**「たぶん伏せ終わっただろう」で撮ると、
/// 伏せる仕組みが在るのに中身が写ります。
const REDACT_TIMEOUT: Duration = Duration::from_secs(3);

/// 撮る相手のウィンドウ。**メインの 1 枚だけ**を撮ります。
const MAIN_WINDOW: &str = "main";

pub struct TauriCapture {
    app: AppHandle,
}

impl TauriCapture {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self { app })
    }

    /// 画面へ「伏せろ／戻せ」と言い、**返事を待つ**。
    async fn tell_screen(&self, on: bool) -> Result<(), String> {
        let (done, wait) = oneshot::channel::<()>();
        let mut done = Some(done);

        // **先に聞き耳を立ててから言う。**逆にすると、速い返事を取りこぼす。
        let handle = self.app.once(REDACT_READY_EVENT, move |_| {
            if let Some(done) = done.take() {
                let _ = done.send(());
            }
        });

        self.app
            .emit(REDACT_EVENT, on)
            .map_err(|error| format!("画面へ伝えられません: {error}"))?;

        match tokio::time::timeout(REDACT_TIMEOUT, wait).await {
            Ok(Ok(())) => Ok(()),
            _ => {
                self.app.unlisten(handle);
                Err(format!(
                    "画面が {} 秒以内に応じませんでした。**伏せ終わっていないので撮りません。**\
                     ウィンドウが最小化されていないか確かめてください",
                    REDACT_TIMEOUT.as_secs()
                ))
            }
        }
    }

    /// ウィンドウを 1 枚撮る。**画面の許可が要ります**（macOS）。
    fn photograph(&self, max_edge: u32, redacted: bool) -> Result<WindowShot, String> {
        let window = self
            .app
            .get_webview_window(MAIN_WINDOW)
            .ok_or_else(|| "sshboard のウィンドウが見つかりません".to_string())?;
        let title = window.title().unwrap_or_else(|_| "sshboard".to_string());

        let found = xcap::Window::all()
            .map_err(|error| {
                format!(
                    "画面を撮れません: {error}。\
                     macOS では「画面収録」の許可が要ります（システム設定 → \
                     プライバシーとセキュリティ → 画面収録）。**人が 1 回許す必要があります。**"
                )
            })?
            .into_iter()
            .find(|found| found.title().map(|held| held == title).unwrap_or(false))
            .ok_or_else(|| {
                "sshboard のウィンドウを画面上で見つけられません。\
                 最小化されていると撮れません（人が戻す必要があります）"
                    .to_string()
            })?;

        let image = found
            .capture_image()
            .map_err(|error| format!("画面を撮れません: {error}"))?;

        let (width, height) = (image.width(), image.height());
        // **引き伸ばさない。**小さい窓を大きく返しても、崩れは見やすくならない。
        let longest = width.max(height);
        let scaled = if longest > max_edge {
            let ratio = f64::from(max_edge) / f64::from(longest);
            let to = |value: u32| ((f64::from(value) * ratio).round() as u32).max(1);
            image::imageops::resize(
                &image,
                to(width),
                to(height),
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            image
        };

        let (scaled_width, scaled_height) = (scaled.width(), scaled.height());
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(scaled)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|error| format!("PNG にできません: {error}"))?;

        Ok(WindowShot {
            png,
            title,
            width,
            height,
            scaled_width,
            scaled_height,
            redacted,
        })
    }
}

impl WindowCapture for TauriCapture {
    fn capture<'a>(
        &'a self,
        redact: bool,
        max_edge: u32,
    ) -> Pin<Box<dyn Future<Output = Result<WindowShot, String>> + Send + 'a>> {
        Box::pin(async move {
            if redact {
                self.tell_screen(true).await?;
            }

            let shot = self.photograph(max_edge, redact);

            // **伏せたまま画面を残さない。**撮影が失敗しても必ず戻す。
            if redact {
                let _ = self.tell_screen(false).await;
            }
            shot
        })
    }
}
