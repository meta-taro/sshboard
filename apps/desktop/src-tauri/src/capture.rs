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

/// macOS の画面収録の許可を尋ねる／求める。
///
/// **許可が無くても撮影は失敗しません。**真っ白な画像が返ってきます（実測・2026-08-31）。
/// エラーで返らないので、**こちらから先に尋ねないと嘘をつく**ことになります。
///
/// `CGRequestScreenCaptureAccess` を呼ぶと、OS が 1 回だけ確認を出し、
/// **以後この実行ファイルがシステム設定の一覧に載ります。**
/// 呼ばないと一覧に出てこないので、人は許可のしようがありません。
#[cfg(target_os = "macos")]
mod screen_permission {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    /// いま許可されているか。**尋ねるだけで、確認は出さない。**
    pub fn granted() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    /// 許可を求める。**OS の確認が 1 回出る**（人が押すまで返らないことはない）。
    pub fn request() -> bool {
        unsafe { CGRequestScreenCaptureAccess() }
    }
}

/// 画素が全部同じ色か。**許可が無いときに返る真っ白を捕まえる。**
///
/// sshboard の窓が 1 色だけ、ということは起こりません。
/// macOS 以外でも、**撮れていないのに撮れたと言わない**ための最後の網です。
fn is_blank(image: &image::RgbaImage) -> bool {
    let mut pixels = image.pixels();
    let Some(first) = pixels.next() else {
        return true;
    };
    pixels.all(|pixel| pixel == first)
}

/// 許可が無いときに返す説明。**次に何をすればよいかまで書く**（§17）。
const PERMISSION_HINT: &str = "画面収録が許可されていないため、撮っても真っ白になります。\
     macOS の「システム設定 → プライバシーとセキュリティ → 画面収録とシステムオーディオ録音」で \
     sshboard を入にしてください。**一覧に sshboard が出ていない場合は、\
     いまこの呼び出しが登録したので、開き直すと出てきます。**\
     許可したあとは sshboard を再起動してください（macOS は起動中のアプリに反映しません）。";

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
        // **撮る前に尋ねる。**許可が無いと真っ白が返るだけで、失敗してくれない。
        #[cfg(target_os = "macos")]
        if !screen_permission::granted() {
            // ここで求めておくと、**システム設定の一覧に sshboard が載る。**
            // 載らないと、人は許可を出しようがない。
            screen_permission::request();
            return Err(PERMISSION_HINT.to_string());
        }

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

        // **撮れていないのに撮れたと言わない。**許可を落とした直後などに起こる。
        if is_blank(&image) {
            return Err(PERMISSION_HINT.to_string());
        }

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
