//! 書き出す／取り込むときの段取り（D18）。
//!
//! 暗号そのものは `roundtrip.rs` で見ています。ここで見るのは
//! **秘密をどこから集め、どこへ戻し、途中で失敗したらどうするか**です。

use std::cell::RefCell;
use std::collections::BTreeMap;

use sshboard_bundle::{apply_payload, build_payload, SecretVault, TransferError};
use sshboard_connections::{ConnectionEntry, Connections};

/// 試験用の保管庫。**実物の OS ストアを触りません。**
#[derive(Default)]
struct FakeVault {
    items: RefCell<BTreeMap<String, String>>,
    /// この名前を入れようとしたら失敗する（巻き戻しの確認用）。
    fail_put_on: Option<String>,
}

impl FakeVault {
    fn with(items: &[(&str, &str)]) -> Self {
        Self {
            items: RefCell::new(
                items
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            ),
            fail_put_on: None,
        }
    }
    fn failing_on(reference: &str) -> Self {
        Self {
            items: RefCell::new(BTreeMap::new()),
            fail_put_on: Some(reference.to_string()),
        }
    }
    fn names(&self) -> Vec<String> {
        self.items.borrow().keys().cloned().collect()
    }
}

impl SecretVault for FakeVault {
    fn get(&self, reference: &str) -> Result<String, String> {
        self.items
            .borrow()
            .get(reference)
            .cloned()
            .ok_or_else(|| format!("無い: {reference}"))
    }
    fn put(&self, reference: &str, secret: &str) -> Result<(), String> {
        if self.fail_put_on.as_deref() == Some(reference) {
            return Err("わざと失敗".into());
        }
        self.items
            .borrow_mut()
            .insert(reference.to_string(), secret.to_string());
        Ok(())
    }
    fn delete(&self, reference: &str) -> Result<(), String> {
        self.items.borrow_mut().remove(reference);
        Ok(())
    }
}

fn entry(id: &str, keyring_ref: Option<&str>) -> ConnectionEntry {
    ConnectionEntry {
        id: id.into(),
        name: id.into(),
        host: format!("{id}.example.com"),
        port: 22,
        user: "someone".into(),
        key_path: Some("/home/me/.ssh/id_ed25519".into()),
        keyring_passphrase_ref: keyring_ref.map(str::to_string),
        keyring_password_ref: None,
        fingerprint: None,
        known_hosts: None,
        color: None,
        tag: None,
        write_roots: vec![],
    }
}

fn store(entries: Vec<ConnectionEntry>) -> Connections {
    Connections {
        version: 1,
        connections: entries,
    }
}

#[test]
fn only_the_ones_that_were_ticked_go_in() {
    // **チェックしたものだけ。**全部入ると、渡すつもりのないサーバーまで渡ります。
    let all = store(vec![entry("a", None), entry("b", None), entry("c", None)]);
    let vault = FakeVault::default();
    let payload = build_payload(&all, &["a".into(), "c".into()], &vault).expect("組めない");
    let ids: Vec<&str> = payload
        .connections
        .connections
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(ids, vec!["a", "c"]);
}

#[test]
fn the_secrets_the_ticked_ones_point_at_are_collected() {
    let all = store(vec![entry("a", Some("a-key")), entry("b", Some("b-key"))]);
    let vault = FakeVault::with(&[("a-key", "AAA"), ("b-key", "BBB")]);
    let payload = build_payload(&all, &["a".into()], &vault).expect("組めない");
    assert_eq!(
        payload.secrets.get("a-key").map(String::as_str),
        Some("AAA")
    );
    // **チェックしていない方の秘密を持っていかない。**
    assert!(!payload.secrets.contains_key("b-key"));
}

#[test]
fn a_connection_that_uses_the_agent_carries_no_secret_and_that_is_not_an_error() {
    // D18: ssh-agent へ委譲している接続は、鍵そのものが入らない。
    // **これは摩擦ではなく、agent へ預けている（D11）ことの自然な帰結。**
    let all = store(vec![entry("a", None)]);
    let vault = FakeVault::default();
    let payload = build_payload(&all, &["a".into()], &vault).expect("組めない");
    assert!(payload.secrets.is_empty());
}

#[test]
fn a_secret_the_store_cannot_give_back_stops_the_export() {
    // **黙って穴の開いたファイルを作らない。**
    // 相手は「入っているはず」で受け取り、繋げないところで初めて気づきます。
    let all = store(vec![entry("a", Some("missing"))]);
    let vault = FakeVault::default();
    assert!(matches!(
        build_payload(&all, &["a".into()], &vault),
        Err(TransferError::SecretMissing { .. })
    ));
}

#[test]
fn an_unknown_id_is_refused_rather_than_quietly_skipped() {
    let all = store(vec![entry("a", None)]);
    let vault = FakeVault::default();
    assert!(matches!(
        build_payload(&all, &["nope".into()], &vault),
        Err(TransferError::UnknownConnection { .. })
    ));
}

#[test]
fn importing_puts_the_secrets_in_before_the_list_is_written() {
    let payload = {
        let mut secrets = BTreeMap::new();
        secrets.insert("a-key".to_string(), "AAA".to_string());
        sshboard_bundle::BundlePayload::new(store(vec![entry("a", Some("a-key"))]), secrets)
    };
    let vault = FakeVault::default();
    let merged = apply_payload(payload, store(vec![]), &vault).expect("取り込めない");
    assert_eq!(vault.get("a-key").as_deref(), Ok("AAA"));
    assert_eq!(merged.connections.len(), 1);
}

#[test]
fn an_incoming_connection_replaces_the_one_with_the_same_id() {
    let payload =
        sshboard_bundle::BundlePayload::new(store(vec![entry("a", None)]), BTreeMap::new());
    let mut existing = entry("a", None);
    existing.host = "old.example.com".into();
    let vault = FakeVault::default();
    let merged = apply_payload(payload, store(vec![existing, entry("z", None)]), &vault)
        .expect("取り込めない");
    assert_eq!(merged.connections.len(), 2, "増やしても減らしてもいけない");
    let a = merged
        .connections
        .iter()
        .find(|e| e.id == "a")
        .expect("a が無い");
    assert_eq!(a.host, "a.example.com", "古いまま残っている");
    assert!(
        merged.connections.iter().any(|e| e.id == "z"),
        "元からあった z を消している"
    );
}

#[test]
fn if_putting_a_secret_fails_the_ones_already_put_are_taken_back_out() {
    // **途中で止まったら、入れた分を戻す。**
    // 半分だけ入った状態は、あとから見て何が起きたか分かりません。
    let mut secrets = BTreeMap::new();
    secrets.insert("first".to_string(), "1".to_string());
    secrets.insert("second".to_string(), "2".to_string());
    let payload = sshboard_bundle::BundlePayload::new(store(vec![entry("a", None)]), secrets);

    let vault = FakeVault::failing_on("second");
    let result = apply_payload(payload, store(vec![]), &vault);

    assert!(matches!(result, Err(TransferError::SecretStore { .. })));
    assert!(
        vault.names().is_empty(),
        "入れた分が残っている: {:?}",
        vault.names()
    );
}

#[test]
fn the_login_password_travels_too() {
    // **鍵のパスフレーズだけ運んで、ログインのパスワードを置いていくと、
    // 渡した相手は繋げません。**バンドルの目的（1 ファイルで渡す・D18）が壊れます。
    let mut with_password = entry("a", None);
    with_password.keyring_password_ref = Some("a-login".into());

    let all = store(vec![with_password]);
    let vault = FakeVault::with(&[("a-login", "hunter2")]);
    let payload = build_payload(&all, &["a".into()], &vault).expect("組めない");

    assert_eq!(
        payload.secrets.get("a-login").map(String::as_str),
        Some("hunter2"),
        "ログインのパスワードが入っていない"
    );
}

#[test]
fn a_missing_login_password_stops_the_export_too() {
    // 鍵のパスフレーズと同じ扱い。**黙って穴の開いたファイルを作らない。**
    let mut with_password = entry("a", None);
    with_password.keyring_password_ref = Some("missing".into());

    let all = store(vec![with_password]);
    let vault = FakeVault::default();
    assert!(matches!(
        build_payload(&all, &["a".into()], &vault),
        Err(TransferError::SecretMissing { .. })
    ));
}
