//! 鍵を選んだあと、**繋ぎに行く前に**何を人へ返すか（D28）。
//!
//! ここはサーバーを一切使いません。`connect` は認証の準備を先に済ませるので、
//! **鍵の判定を間違えていることは、繋がる前に分かります。**
//!
//! 見張るのは 2 つです。
//!
//! 1. **パスフレーズが要る鍵で、黙って失敗しない**（PPK がまさにこれだった）
//! 2. **公開鍵を指したときに、そう言う**（取り違えが一番多い）

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sshboard_band::{Actor, Band};
use sshboard_engine::{Engine, EngineError};
use sshboard_stream::OutputStream;

mod common;
use common::toml_string;

/// 鍵のパスだけを書いた接続一覧。**繋ぎに行く前に断られる想定。**
fn registry_with_key(dir: &tempfile::TempDir, key: &Path) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let toml = format!(
        "version = 1\n\n[[connections]]\nid = \"pending\"\nname = \"Pending\"\n\
         host = \"127.0.0.1\"\nport = 65000\nuser = \"nobody\"\n\
         key_path = \"{}\"\n",
        toml_string(key)
    );
    std::fs::write(&path, toml).expect("接続一覧を書けない");
    path
}

fn engine_at(path: PathBuf) -> Engine {
    Engine::new(Band::new(), Arc::new(OutputStream::new()), path)
}

/// 鍵ファイルを 1 つ置く。**中身は見出しだけ**（本物の鍵ではありません）。
fn key_file(dir: &tempfile::TempDir, name: &str, contents: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, contents).expect("鍵ファイルを書けない");
    path
}

#[tokio::test]
async fn an_encrypted_putty_key_asks_for_the_passphrase() {
    // **これが直したかった穴です。**PPK を「パスフレーズ不要」と見て、
    // 何も聞かずに認証へ行き、読めない理由が人に伝わらなかった。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        // **わざと OpenSSH らしい名前にする。**拡張子を見ていたら間違える。
        "looks-like-openssh.id_ed25519",
        "PuTTY-User-Key-File-3: ssh-ed25519\r\nEncryption: aes256-cbc\r\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    let result = engine.connect(Actor::Human, "pending", None).await;

    assert!(
        matches!(result, Err(EngineError::PassphraseNeeded { .. })),
        "PPK のパスフレーズを聞いていない: {:?}",
        result.map(|open| open.id)
    );
}

#[tokio::test]
async fn a_putty_key_without_a_passphrase_is_not_asked_for_one() {
    // 聞くこと自体が壁になります。**何を入れればいいのか分からない画面**を出さない。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        "plain.ppk",
        "PuTTY-User-Key-File-3: ssh-ed25519\r\nEncryption: none\r\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    let result = engine.connect(Actor::Human, "pending", None).await;

    assert!(
        !matches!(result, Err(EngineError::PassphraseNeeded { .. })),
        "素の鍵にパスフレーズを聞いている"
    );
}

#[tokio::test]
async fn pointing_at_a_public_key_says_so_instead_of_failing_to_authenticate() {
    // `.pub` の取り違えは一番多い。**「認証できません」だけでは人は直せません。**
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        "id_ed25519.pub",
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI0000 someone@example\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    let result = engine.connect(Actor::Human, "pending", None).await;

    match result {
        Err(EngineError::UnusableKey { format, .. }) => {
            assert_eq!(format, "public key");
        }
        other => panic!(
            "公開鍵をそのまま使おうとしている: {:?}",
            other.map(|open| open.id)
        ),
    }
}

#[tokio::test]
async fn the_refusal_never_carries_the_path_to_the_key() {
    // **鍵のパスは接続先の情報です**（CLAUDE.md 禁止事項 4）。
    // 画面にも記録にも出さない。出す名前は形式だけ。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(&dir, "secret-place.pub", "ssh-ed25519 AAAAC3Nza0000 x@y\n");
    let engine = engine_at(registry_with_key(&dir, &key));

    let Err(error) = engine.connect(Actor::Human, "pending", None).await else {
        panic!("断っていない");
    };

    let shown = error.to_string();
    assert!(
        !shown.contains("secret-place") && !shown.contains(dir.path().to_str().unwrap()),
        "断り文にパスが混ざっている: {shown}"
    );
}

#[tokio::test]
async fn a_windows_style_path_does_not_corrupt_the_connection_list() {
    // **CI（windows-latest）で実際に落ちた穴です。**
    // `C:\Users\...` を TOML へそのまま書くと、`\U` が unicode エスケープとして
    // 読まれ、`invalid unicode 8-digit hex code` で**接続一覧ごと壊れました。**
    //
    // 製品側は `toml::to_string_pretty` を通すので無事でしたが、
    // **テストが手で組んでいたため、Windows でだけ落ちていました。**
    //
    // `\` を含むパス文字列は Unix でも作れるので、**ここで再現できます。**
    // Windows を待たずに気づけるようにしておきます。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let windows_ish = Path::new(r"C:\Users\RUNNER~1\AppData\Local\Temp\id_ed25519");
    let engine = engine_at(registry_with_key(&dir, windows_ish));

    let Err(error) = engine.connect(Actor::Human, "pending", None).await else {
        panic!("在りもしない鍵で繋ごうとしている");
    };

    // **TOML が壊れていないこと。**壊れていれば「接続の一覧を読めません」になる。
    // 鍵そのものが無いことで断られるのが正しい。
    let shown = error.to_string();
    assert!(
        !shown.contains("TOML") && !shown.contains("一覧を読めません"),
        "接続一覧が壊れている（パスの `\\` が TOML を壊した）: {shown}"
    );
}

// --- 繋ぐ前に落ちたことが記録に残るか（Issue #13） ------------------------
//
// 実機の指摘（2026-09-09・実作業の最中）:
//
// > `diagnostics(limit=20)` を呼びました。`{"events":[],"kept":0}` **空です。**
// > `diagnostics` は AI が自分で状況を掴むための唯一の窓口ですが、
// > そこが空だと、AI は人に画面を見てもらうしかありません。
//
// **そのとおりでした。**`auth_for` には `self.diag` の呼び出しが 1 つも
// ありませんでした。しかも `connect` はこの判定を **SSH を張るより前**に
// 行うので、`reach` / `host-key` / `auth` の行も 1 本も出ません。
//
// **書く場所が無かった**というのが正確な言い方です。

/// 記録の全文。**失敗メッセージにそのまま出す。**
fn rendered(engine: &Engine) -> String {
    engine
        .diagnostics()
        .recent(50)
        .iter()
        .map(|event| event.render())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn stopping_for_a_passphrase_is_written_down() {
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        "needs-a-passphrase.ppk",
        "PuTTY-User-Key-File-3: ssh-ed25519\r\nEncryption: aes256-cbc\r\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    // Act
    let refused = engine.connect(Actor::Ai, "pending", None).await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::PassphraseNeeded { .. })),
        "断り方が違う: {refused:?}"
    );

    let events = engine.diagnostics().recent(50);
    assert!(
        !events.is_empty(),
        "**繋ぐ前に落ちたのに、記録が 1 行も無い。**\
         `diagnostics` は「まずこれを呼べ」と説明しているのに空を返す"
    );

    let stopped = events
        .iter()
        .find(|event| event.level == sshboard_diag::Level::Error)
        .unwrap_or_else(|| panic!("失敗として残っていない:\n{}", rendered(&engine)));

    // **どの段階まで進んだかが分かること**（ご要望どおり）。
    // 鍵は読めています — 止まったのはパスフレーズ待ちです。
    assert_eq!(stopped.stage, sshboard_diag::Stage::Auth);
    assert_eq!(stopped.connection.as_deref(), Some("pending"));
    // **「駄目でした」で終わらせない**（product-baseline §17）。
    assert!(
        stopped.hint.is_some(),
        "次の一手が付いていない:\n{}",
        rendered(&engine)
    );
}

#[tokio::test]
async fn an_unusable_key_is_written_down_too() {
    // 公開鍵を指した、という一番多い取り違え。**これも残らなければ追えません。**
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        "id_ed25519.pub",
        "ssh-ed25519 AAAAC3Nz nobody@example\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    // Act
    let refused = engine.connect(Actor::Ai, "pending", None).await;

    // Assert
    assert!(
        matches!(refused, Err(EngineError::UnusableKey { .. })),
        "断り方が違う: {refused:?}"
    );
    assert!(
        engine
            .diagnostics()
            .recent(50)
            .iter()
            .any(|event| event.level == sshboard_diag::Level::Error),
        "使えない鍵を断った記録が無い:\n{}",
        rendered(&engine)
    );
}

#[tokio::test]
async fn the_record_never_carries_the_host_or_the_user() {
    // **識別子までは出してよい。ホスト・利用者・パスは出さない**（PRD §8）。
    // Issue #13 の補足で、この線引きを確かめられた所です。
    // Arrange
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let key = key_file(
        &dir,
        "needs-a-passphrase.ppk",
        "PuTTY-User-Key-File-3: ssh-ed25519\r\nEncryption: aes256-cbc\r\n",
    );
    let engine = engine_at(registry_with_key(&dir, &key));

    // Act
    let _ = engine.connect(Actor::Ai, "pending", None).await;

    // Assert
    let written = rendered(&engine);
    assert!(written.contains("pending"), "識別子まで消してしまっている");
    for leak in ["127.0.0.1", "nobody", "65000"] {
        assert!(
            !written.contains(leak),
            "記録に接続先が入っている（{leak}）:\n{written}"
        );
    }
}
