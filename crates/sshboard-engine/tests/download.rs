//! ダウンロード（サーバー → 手元）のうち、**サーバーが要らない部分**。
//!
//! ここで見張るのは 1 つだけです。**手元にあるものを黙って上書きしない。**
//! 上げる側と違い、落とす側が壊すのは**人の手元のファイル**で、
//! sshboard からは元へ戻せません（product-baseline §13）。

use std::path::PathBuf;
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError, OnConflict};
use sshboard_stream::OutputStream;

fn engine_at(path: PathBuf) -> Engine {
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

#[tokio::test]
async fn an_existing_local_file_is_refused_before_the_server_is_touched() {
    // **繋がっていないのに `Local` が返ること**が要点です。
    // これは「サーバーへ行く前に断った」という意味で、
    // 断ったのに中身が半分書き換わっている、が起きません。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    let local = dir.path().join("already-here.txt");
    std::fs::write(&local, "人が置いたもの".as_bytes()).expect("手元に書けない");

    let result = engine
        .download_file(Actor::Human, "/etc/hostname", &local, OnConflict::Refuse)
        .await;

    assert!(matches!(result, Err(EngineError::Local(_))), "{result:?}");
    assert_eq!(
        std::fs::read(&local).expect("読めない"),
        "人が置いたもの".as_bytes(),
        "断ったのに手元のファイルが変わっている"
    );
}

#[tokio::test]
async fn overwriting_happens_only_when_it_was_asked_for() {
    // 上書きを頼まれたときは、手元の検査を抜けて**接続の有無で断る**。
    // ここが `Local` のままだと、人が「上書きする」を選んでも永遠に落とせない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    let local = dir.path().join("already-here.txt");
    std::fs::write(&local, b"x").expect("手元に書けない");

    let result = engine
        .download_file(Actor::Human, "/etc/hostname", &local, OnConflict::Overwrite)
        .await;

    assert!(
        matches!(result, Err(EngineError::NotConnected)),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_destination_whose_directory_is_missing_is_a_local_problem() {
    // どちら側で失敗したのかが分からないと、人は直せない（§17）。
    // 無い階層は**勝手に作りません**。人が見ている画面の話なので、
    // 落とし先が思っていた場所と違うことに気づけなくなる方が危ない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let engine = engine_at(dir.path().join("connections.toml"));

    let result = engine
        .download_file(
            Actor::Human,
            "/etc/hostname",
            &dir.path().join("no-such-dir").join("x.txt"),
            OnConflict::Refuse,
        )
        .await;

    assert!(matches!(result, Err(EngineError::Local(_))), "{result:?}");
}
