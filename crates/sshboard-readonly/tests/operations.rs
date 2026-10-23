//! 状態を変える操作の一覧（D45 / Issue #17）。**サーバーを一切使いません。**
//!
//! `readonly.toml` は**読み取り専用のまま触りません。**別ファイルにします。
//!
//! > **`readonly` という名前のファイルに書き込み操作が入ります。**
//! > 「約束はゲートではない」で排除したものが、名前だけのゲートとして戻ってきます
//!
//! **指摘のとおり**なので、**名前で区別がつく**ことを要点にしています。
//!
//! 見張るのは 6 つ。
//!
//! 1. **AI が渡せるのは id だけ**（D3 —— 引数のスロットを作らない）
//! 2. **既定は空。**人が書くまで 1 本も走らない
//! 3. **回数で止まる。**暴走しても上限で止まる
//! 4. **踏ませないものは、書いても弾く**（設定で有効にできない）
//! 5. **読めない一覧を、空として扱わない**（「許可したのに断られる」を作らない）
//! 6. **同じ id を 2 つ書けない**（どちらが走るのか読めなくなる）

use sshboard_readonly::{Operations, OperationsError};

const SAMPLE: &str = r#"
version = 1

[[operation]]
id = "restart-httpd"
run = "sudo systemctl restart httpd"
description = "Apache を再起動する"
max_per_hour = 3

[[operation]]
id = "reload-httpd"
run = "sudo systemctl reload httpd"
description = "Apache に設定を読み直させる（無停止）"
max_per_hour = 6
"#;

#[test]
fn nothing_runs_until_a_person_writes_the_file() {
    // **既定は空**（D3 と同じ立場）。約束ではなく、**空であること**がゲートです。
    let empty = Operations::empty();

    assert!(empty.is_empty());
    assert!(empty.get("restart-httpd").is_none());
}

#[test]
fn an_operation_is_reached_by_id_only() {
    // **AI が渡せるのは id だけ。**引数のスロットを作りません（D3）。
    let listed = Operations::parse(SAMPLE).expect("読めない");

    let found = listed.get("restart-httpd").expect("在るはずのものが無い");
    assert_eq!(found.run, "sudo systemctl restart httpd");
    assert_eq!(found.max_per_hour, 3);
    assert!(listed.get("rm -rf /").is_none(), "id 以外で引けてしまう");
}

#[test]
fn what_must_never_run_is_refused_even_when_written_down() {
    // **設定で有効にできない**（D47 の 6）。dbboard が `GRANT` / `DROP` を
    // 恒久的に断っているのと同じ形です。**AI が自分の檻を広げられない。**
    for forbidden in [
        // 檻そのものを書き換える
        "echo x >> /etc/sudoers",
        "visudo",
        // 鍵と利用者
        "useradd attacker",
        "usermod -aG wheel someone",
        "cat ~/.ssh/id_ed25519",
        // **パスワードの入れ物**（D48 で新しく届くようになった所）。
        // `become = "ask"` を入れるまで、root しか読めないものは
        // **そもそも読めませんでした。**読めるようになった以上、ここも塞ぎます。
        "sudo cat /etc/shadow",
        "sudo tail /etc/gshadow",
        // 壊す
        "rm -rf /var",
        "mkfs.ext4 /dev/sda1",
        "dd if=/dev/zero of=/dev/sda",
    ] {
        let toml = format!(
            "version = 1\n\n[[operation]]\nid = \"bad\"\nrun = \"{forbidden}\"\n\
             description = \"x\"\nmax_per_hour = 1\n"
        );
        let refused = Operations::parse(&toml);
        assert!(
            matches!(refused, Err(OperationsError::NeverAllowed { .. })),
            "**踏ませてはいけないものが通っている**: {forbidden} → {refused:?}"
        );
    }
}

#[test]
fn a_broken_file_is_not_treated_as_an_empty_one() {
    // **空として扱わない。**扱うと「書いたのに断られる」になり、
    // **原因が書き間違いだと誰も気づけません**（`readonly.toml` と同じ判断）。
    let refused = Operations::parse("version = 1\n[[operation]]\nid = \"only-an-id\"\n");

    assert!(
        matches!(refused, Err(OperationsError::Malformed { .. })),
        "壊れた一覧を空として通している: {refused:?}"
    );
}

#[test]
fn an_operation_needs_a_ceiling_a_person_can_read() {
    // **上限の無い操作を書けない。**暴走しても止まる所が要ります。
    let refused = Operations::parse(
        "version = 1\n\n[[operation]]\nid = \"x\"\nrun = \"systemctl reload httpd\"\n\
         description = \"x\"\nmax_per_hour = 0\n",
    );

    assert!(
        matches!(refused, Err(OperationsError::Malformed { .. })),
        "1 時間あたり 0 回の操作を通している: {refused:?}"
    );
}

#[test]
fn the_same_id_cannot_be_written_twice() {
    // 同じ id が 2 つあると、**どちらが走るのか人が読めません。**
    let refused = Operations::parse(
        "version = 1\n\n[[operation]]\nid = \"x\"\nrun = \"systemctl reload httpd\"\n\
         description = \"a\"\nmax_per_hour = 1\n\n\
         [[operation]]\nid = \"x\"\nrun = \"systemctl restart httpd\"\n\
         description = \"b\"\nmax_per_hour = 1\n",
    );

    assert!(
        matches!(refused, Err(OperationsError::Malformed { .. })),
        "同じ id を 2 つ通している: {refused:?}"
    );
}

/// **同梱したひな型が、実際に通ること**（実運用の報告・2026-10-23）。
///
/// `docs/operations.example.toml` は「使うものだけ写してください」という棚です。
/// **手で書いた TOML が `parse` を通らなければ、棚として役に立ちません** ——
/// 写した人が「書いたのに断られる」に当たり、
/// **原因が写し元の書き間違いだと誰も気づけません。**
///
/// 本番で起きたのは、書く場所が分からず**画面の外で操作した**ことでした。
/// 棚を置く以上、**その棚が壊れていないこと**は機械が見ます。
#[test]
fn the_shipped_examples_actually_parse() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/operations.example.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} が読めません: {error}", path.display()));

    let parsed = Operations::parse(&text).expect("同梱したひな型が通らない");

    // **空として通っていないこと。**0 件でも `parse` は通るので、数えます。
    assert!(
        parsed.all().len() >= 8,
        "ひな型が少なすぎます（{} 件）—— 読み方が壊れている可能性",
        parsed.all().len()
    );
}

/// **ひな型に、戻す道が入っていること。**
///
/// 直す操作だけを並べると、**人は戻せないまま当てます。**
/// 本番で必要だった 4 つのうち、最初が「conf を日付付きでバックアップ」でした。
#[test]
fn the_examples_include_a_way_back() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/operations.example.toml");
    let text = std::fs::read_to_string(&path).expect("読めない");
    let parsed = Operations::parse(&text).expect("通らない");

    assert!(
        parsed.all().iter().any(|op| op.id.starts_with("backup-")),
        "控えを取る操作が入っていません"
    );
}

/// **試しと本番を同じ id にしていないこと。**
///
/// certbot の本番取得は回数制限が厳しく、同じ id だと
/// **試したつもりで本番の枠を使います。**
#[test]
fn the_examples_keep_the_dry_run_apart() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/operations.example.toml");
    let text = std::fs::read_to_string(&path).expect("読めない");
    let parsed = Operations::parse(&text).expect("通らない");

    let dry = parsed
        .all()
        .iter()
        .find(|op| op.run.contains("--dry-run"))
        .expect("試しの操作が無い");
    let real = parsed
        .all()
        .iter()
        .find(|op| op.run.contains("certonly"))
        .expect("本番の操作が無い");

    assert_ne!(dry.id, real.id, "試しと本番が同じ id になっています");
}

/// **ひな型が、踏ませないものを含んでいないこと。**
///
/// `parse` が弾くので通らないはずですが、**棚に書いてあるだけで
/// 人は写します。**写してから弾かれるのは、置いた側の落ち度です。
#[test]
fn the_examples_contain_nothing_that_is_never_allowed() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/operations.example.toml");
    let text = std::fs::read_to_string(&path).expect("読めない");

    // `parse` が通っている＝NEVER を含まない、が成り立つ。
    // ここでは**注記の中**も見ます（人は注記からも写します）。
    for word in ["rm -rf", "mkfs", "id_ed25519", "authorized_keys"] {
        for line in text.lines() {
            let line = line.trim_start();
            // 「入れられないもの」の説明行は除く。
            if line.starts_with('#') {
                continue;
            }
            assert!(
                !line.contains(word),
                "ひな型の中身に `{word}` が入っています: {line}"
            );
        }
    }
}
