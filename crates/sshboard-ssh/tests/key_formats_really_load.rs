//! **判定と実物を突き合わせる。**
//!
//! `inspect_key` は見出しだけを見て「使える / パスフレーズが要る」と言います。
//! **その言い分が `russh` の実際の挙動と食い違っていたら、製品が嘘をつきます。**
//! 食い違いの向きで、人が受ける被害が変わります。
//!
//! - 「使える」と言って読めない → 繋いだ先で理由の分からない失敗
//! - 「パスフレーズ不要」と言って要る → **何も聞かずに失敗する**（実際に起きていた）
//! - 「パスフレーズが要る」と言って要らない → 何を入れればいいか分からない画面
//!
//! 鍵はここで**その場で作って捨てます**。リポジトリにも手元にも残しません。

use std::path::{Path, PathBuf};
use std::process::Command;

use russh::keys::decode_secret_key;
use sshboard_ssh::inspect_key;

fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--help")
        .output()
        .map(|out| out.status.success() || !out.stderr.is_empty())
        .unwrap_or(false)
}

/// 使い捨ての鍵を 1 本作る。**パスフレーズは引数のものだけ。**
fn keygen(dir: &Path, name: &str, args: &[&str], passphrase: &str) -> PathBuf {
    let path = dir.join(name);
    let done = Command::new("ssh-keygen")
        .arg("-q")
        .args(args)
        .args(["-N", passphrase, "-f"])
        .arg(&path)
        .output()
        .expect("ssh-keygen を起動できない");
    assert!(
        done.status.success(),
        "{name} を作れない: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    path
}

/// OpenSSH 形式の鍵を PuTTY 形式へ書き出す。**確認のためだけに作ります。**
fn to_ppk(dir: &Path, source: &Path, name: &str, version: u8, passphrase: Option<&str>) -> PathBuf {
    let path = dir.join(name);
    let mut command = Command::new("puttygen");
    command
        .arg(source)
        .args(["-O", "private", "--ppk-param"])
        .arg(format!("version={version}"));

    let pass_file = dir.join(format!("{name}.pass"));
    if let Some(pass) = passphrase {
        std::fs::write(&pass_file, pass).expect("パスフレーズを書けない");
        command.arg("--new-passphrase").arg(&pass_file);
    }
    let done = command.arg("-o").arg(&path).output().expect("puttygen");
    // **書いたパスフレーズはその場で捨てる。**
    let _ = std::fs::remove_file(&pass_file);
    assert!(
        done.status.success(),
        "{name} を作れない: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    path
}

/// 判定と実物が一致しているか、1 本ずつ確かめる。
fn agrees(path: &Path, passphrase: Option<&str>) {
    let bytes = std::fs::read(path).expect("読めない");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let facts = inspect_key(&bytes);
    let name = path.file_name().unwrap().to_string_lossy();

    // 1. 「使える」と言ったなら、正しいパスフレーズで**本当に読める**こと。
    let loaded = decode_secret_key(&text, passphrase).is_ok();
    assert_eq!(
        facts.usable(),
        loaded,
        "{name}（{}）: 使えると言った = {} / 実際に読めた = {}",
        facts.format.label(),
        facts.usable(),
        loaded
    );

    // 2. 「パスフレーズが要る」と言ったなら、**無しでは読めない**こと。
    //    逆に「要らない」と言ったなら、無しで読めること。
    if facts.usable() {
        let without = decode_secret_key(&text, None).is_ok();
        assert_eq!(
            facts.needs_passphrase,
            !without,
            "{name}（{}）: 要ると言った = {} / 無しで読めた = {}",
            facts.format.label(),
            facts.needs_passphrase,
            without
        );
    }
}

#[test]
fn what_the_product_claims_about_a_key_matches_what_russh_can_actually_read() {
    if !have("ssh-keygen") || !have("puttygen") {
        println!("ssh-keygen / puttygen がありません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let at = dir.path();

    // OpenSSH 形式（素・パスフレーズ付き）
    let ed_plain = keygen(at, "ed_plain", &["-t", "ed25519"], "");
    agrees(&ed_plain, None);
    agrees(
        &keygen(at, "ed_enc", &["-t", "ed25519"], "sshboard-pass"),
        Some("sshboard-pass"),
    );

    // 古い PEM（PKCS#1）。実機に残っている（Issue 002）。
    let pem = ["-t", "rsa", "-b", "2048", "-m", "PEM"];
    agrees(&keygen(at, "pem_plain", &pem, ""), None);
    agrees(
        &keygen(at, "pem_enc", &pem, "sshboard-pass"),
        Some("sshboard-pass"),
    );

    // PKCS#8
    let p8 = ["-t", "rsa", "-b", "2048", "-m", "PKCS8"];
    agrees(&keygen(at, "p8_plain", &p8, ""), None);
    agrees(
        &keygen(at, "p8_enc", &p8, "sshboard-pass"),
        Some("sshboard-pass"),
    );

    // PuTTY 形式 v2 / v3、素とパスフレーズ付き。**変換せずにそのまま。**
    agrees(&to_ppk(at, &ed_plain, "v2_plain.ppk", 2, None), None);
    agrees(
        &to_ppk(at, &ed_plain, "v2_enc.ppk", 2, Some("sshboard-pass")),
        Some("sshboard-pass"),
    );
    agrees(&to_ppk(at, &ed_plain, "v3_plain.ppk", 3, None), None);
    agrees(
        &to_ppk(at, &ed_plain, "v3_enc.ppk", 3, Some("sshboard-pass")),
        Some("sshboard-pass"),
    );
}

#[test]
fn a_public_key_is_refused_and_russh_agrees_that_it_is_not_a_key() {
    if !have("ssh-keygen") {
        println!("ssh-keygen がありません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let private = keygen(dir.path(), "pair", &["-t", "ed25519"], "");
    let public = private.with_extension("pub");

    agrees(&public, None);
    assert!(!inspect_key(&std::fs::read(&public).unwrap()).usable());
}

/// `openssl` で作った鍵。**`ssh-keygen` では作れない形**を確かめるため。
fn openssl_key(dir: &Path, source: &Path, name: &str, extra: &[&str]) -> Option<PathBuf> {
    let path = dir.join(name);
    let done = Command::new("openssl")
        .args(["rsa", "-in"])
        .arg(source)
        .args(["-aes256", "-passout", "pass:sshboard-pass", "-out"])
        .arg(&path)
        .args(extra)
        .output()
        .ok()?;
    done.status.success().then_some(path)
}

/// 暗号化された PKCS#8 を、方式を指定して 1 本作る。
///
/// **`ssh-keygen -m PKCS8` では方式を選べません。**しかも既定が OS で違い、
/// macOS（LibreSSL）は PRF に hmacWithSHA1、Linux / Windows（OpenSSL 3）は
/// hmacWithSHA256 を使います。**同じコマンドが別の鍵を作る。**
/// ここを実測に使うと、テストが「その OS で偶然できた鍵」を測ることになります。
fn pkcs8_with(dir: &Path, source: &Path, name: &str, extra: &[&str]) -> Option<PathBuf> {
    let path = dir.join(name);
    let done = Command::new("openssl")
        .args(["pkcs8", "-topk8", "-in"])
        .arg(source)
        .args(["-passout", "pass:sshboard-pass", "-out"])
        .arg(&path)
        .args(extra)
        .output()
        .ok()?;
    done.status.success().then_some(path)
}

#[test]
fn a_key_russh_cannot_read_is_refused_up_front_rather_than_failing_later() {
    // **「使えます」と言って読めないのが一番たちが悪い。**
    // 繋いだ先で理由の分からない失敗になり、人は鍵を作り直しはじめる。
    //
    // **その逆も同じだけ悪い**（D28 の「止めるべき条件」）。
    // 読める鍵を「使えません」と断ると、正しい鍵が丸ごと行き止まりになる。
    // 一律で断っていた頃、**Linux / Windows の ssh-keygen が作る鍵が全部これでした。**
    if !have("ssh-keygen") {
        println!("ssh-keygen がありません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let at = dir.path();

    // 1. その OS の ssh-keygen が作るものは、**どちらに転んでも判定と一致すること。**
    //    ここは「読める / 読めない」を決め打ちしない。OS で変わるため。
    let p8 = ["-t", "rsa", "-b", "2048", "-m", "PKCS8"];
    agrees(
        &keygen(at, "p8_enc", &p8, "sshboard-pass"),
        Some("sshboard-pass"),
    );

    // 2. 方式を名指しして、**実測表のとおりに分かれること。**
    if !have("openssl") {
        println!("openssl がありません（想定内・ここまで）");
        return;
    }
    let plain = keygen(at, "plain", &["-t", "rsa", "-b", "2048", "-m", "PEM"], "");

    // 読めない側 — PRF が hmacWithSHA1（省略時の既定でもある）。
    for (name, extra) in [
        ("sha1", vec!["-v2", "aes-256-cbc", "-v2prf", "hmacWithSHA1"]),
        (
            "3des",
            vec!["-v2", "des-ede3-cbc", "-v2prf", "hmacWithSHA256"],
        ),
        ("pbes1", vec!["-v1", "PBE-SHA1-3DES"]),
    ] {
        let Some(path) = pkcs8_with(at, &plain, &format!("p8_{name}"), &extra) else {
            println!("openssl が {name} を作れません（想定内・飛ばします）");
            continue;
        };
        agrees(&path, Some("sshboard-pass"));
        assert!(
            !inspect_key(&std::fs::read(&path).unwrap()).usable(),
            "{name}: 読めないのに「使える」と言っている"
        );
    }

    // 読める側 — **ここを断ってはいけない。**
    for (name, extra) in [
        (
            "sha256",
            vec!["-v2", "aes-256-cbc", "-v2prf", "hmacWithSHA256"],
        ),
        (
            "sha512",
            vec!["-v2", "aes-128-cbc", "-v2prf", "hmacWithSHA512"],
        ),
    ] {
        let Some(path) = pkcs8_with(at, &plain, &format!("p8_{name}"), &extra) else {
            println!("openssl が {name} を作れません（想定内・飛ばします）");
            continue;
        };
        agrees(&path, Some("sshboard-pass"));
        assert!(
            inspect_key(&std::fs::read(&path).unwrap()).usable(),
            "{name}: 読めるのに断っている（D28 の「止めるべき条件」）"
        );
    }
}

#[test]
fn a_pem_encrypted_with_aes_256_is_judged_by_what_russh_actually_does() {
    // `russh` の PKCS#5 復号には `Aes256Cbc => unimplemented!()` が在る。
    // **落ちるのか、断るのか、読めるのか**で、製品の正解が変わる:
    //
    // - 落ちる  → 渡してはいけない。**製品が落ちる**
    // - 読めない → 断る。理由を「暗号方式に未対応」として出す
    // - 読める  → **断ってはいけない**（D28 の「止めるべき条件」そのもの）
    if !have("ssh-keygen") || !have("openssl") {
        println!("ssh-keygen / openssl がありません（想定内・飛ばします）");
        return;
    }
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let plain = keygen(
        dir.path(),
        "r",
        &["-t", "rsa", "-b", "2048", "-m", "PEM"],
        "",
    );
    let Some(aes256) = openssl_key(dir.path(), &plain, "r_trad", &["-traditional"]) else {
        println!("openssl が -traditional を受け付けません（想定内・飛ばします）");
        return;
    };

    let bytes = std::fs::read(&aes256).expect("読めない");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let claimed = inspect_key(&bytes).usable();

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| decode_secret_key(&text, Some("sshboard-pass")));
    std::panic::set_hook(previous);

    match outcome {
        Err(_) => assert!(!claimed, "russh が落ちる鍵を「使える」と言っている"),
        Ok(Ok(_)) => assert!(
            claimed,
            "russh は読めるのに断っている。**過剰な拒否**（D28 の止めるべき条件）"
        ),
        Ok(Err(error)) => assert!(
            !claimed,
            "russh が読めない鍵を「使える」と言っている（{error}）"
        ),
    }
}
