//! OS 資格情報ストアのテスト。
//!
//! **この製品にとって一番危ない罠を、ここで見張ります**（dbboard ADR-0033）:
//! `keyring` は既定のバックエンドが無いと **in-memory の mock** に落ち、
//! **書き込みが `Ok` を返すのに永続化されません。**
//!
//! **`Entry` を作り直してから読む**こと。同じハンドルを使い回すと mock でも通り、
//! テストが素通りします。

use sshboard_credentials::SecretStore;

/// OS のストアを汚さないよう、この製品と分かる区分名を使う。
const TEST_SERVICE: &str = "sshboard-test-suite";

#[test]
fn a_secret_written_through_one_handle_is_readable_through_a_new_one() {
    // **これが mock 検出テストです。**同じハンドルで読み直すと mock でも通ってしまう。
    // Arrange
    let reference = "phase0-roundtrip";
    let writer = SecretStore::new(TEST_SERVICE);
    let _ = writer.delete(reference);

    // Act
    writer
        .put(reference, "not-a-real-secret")
        .expect("書けない");
    let reader = SecretStore::new(TEST_SERVICE); // ← 作り直す
    let read_back = reader.get(reference);

    // 後始末は assert の前に済ませる。落ちても OS に残さない。
    let _ = reader.delete(reference);

    // Assert
    assert_eq!(
        read_back.expect("読めない — keyring が mock に落ちている可能性があります"),
        "not-a-real-secret"
    );
}

#[test]
fn asking_for_something_that_was_never_stored_says_so() {
    // Arrange
    let store = SecretStore::new(TEST_SERVICE);
    let reference = "phase0-never-stored";
    let _ = store.delete(reference);

    // Act
    let result = store.get(reference);

    // Assert
    assert!(
        matches!(
            result,
            Err(sshboard_credentials::SecretError::NotFound { .. })
        ),
        "実際: {result:?}"
    );
}

#[test]
fn a_deleted_secret_is_gone() {
    // Arrange
    let store = SecretStore::new(TEST_SERVICE);
    let reference = "phase0-delete";
    store.put(reference, "temporary").expect("書けない");

    // Act
    store.delete(reference).expect("消せない");
    let after = SecretStore::new(TEST_SERVICE).get(reference);

    // Assert
    assert!(after.is_err(), "消したのに読めています");
}

#[test]
fn an_error_never_carries_the_secret_itself() {
    // ログや Issue に貼られる場所なので、秘密を載せない（product-baseline §14）。
    // Arrange
    let store = SecretStore::new(TEST_SERVICE);

    // Act
    let error = store.get("phase0-absent").expect_err("あるはずがない");
    let rendered = format!("{error}");

    // Assert
    assert!(
        rendered.contains("phase0-absent"),
        "どの参照かは出す: {rendered}"
    );
    assert!(!rendered.contains("password"), "実際: {rendered}");
}
