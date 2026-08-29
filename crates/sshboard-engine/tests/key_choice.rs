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

/// 鍵のパスだけを書いた接続一覧。**繋ぎに行く前に断られる想定。**
fn registry_with_key(dir: &tempfile::TempDir, key: &Path) -> PathBuf {
    let path = dir.path().join("connections.toml");
    let toml = format!(
        "version = 1\n\n[[connections]]\nid = \"pending\"\nname = \"Pending\"\n\
         host = \"127.0.0.1\"\nport = 65000\nuser = \"nobody\"\n\
         key_path = \"{}\"\n",
        key.display()
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
