//! ssh-agent の返事を読む部分のテスト。
//!
//! **agent を立てずに走ります。**バイト列の解釈だけを見るので、
//! Windows でも macOS でも同じように通ります。

use sshboard_credentials::{
    fingerprint, parse_identities, request_identities, AgentError, AgentIdentity,
};

const SSH_AGENT_IDENTITIES_ANSWER: u8 = 12;

/// `string` は uint32(長さ) + 本体。
fn ssh_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(bytes);
    out
}

fn answer_with(keys: &[(&[u8], &str)]) -> Vec<u8> {
    let mut out = vec![SSH_AGENT_IDENTITIES_ANSWER];
    out.extend_from_slice(&(keys.len() as u32).to_be_bytes());
    for (blob, comment) in keys {
        out.extend_from_slice(&ssh_string(blob));
        out.extend_from_slice(&ssh_string(comment.as_bytes()));
    }
    out
}

#[test]
fn the_request_is_a_five_byte_frame() {
    // Arrange & Act
    let request = request_identities();

    // Assert — 長さ 4 バイト（値 1）＋ 種類 1 バイト（11）
    assert_eq!(request, vec![0, 0, 0, 1, 11]);
}

#[test]
fn every_identity_the_agent_reports_is_parsed() {
    // Arrange
    let payload = answer_with(&[(b"blob-one", "work laptop"), (b"blob-two", "backup key")]);

    // Act
    let identities = parse_identities(&payload).expect("読めない");

    // Assert
    assert_eq!(identities.len(), 2);
    assert_eq!(identities[0].comment, "work laptop");
    assert_eq!(identities[1].comment, "backup key");
    assert!(
        identities[0].fingerprint.starts_with("SHA256:"),
        "{:?}",
        identities[0]
    );
}

#[test]
fn an_agent_with_no_keys_yields_an_empty_list_not_an_error() {
    // 鍵を 1 本も入れていないのは異常ではない。
    let identities = parse_identities(&answer_with(&[])).expect("読めない");

    assert!(identities.is_empty());
}

#[test]
fn a_reply_of_the_wrong_kind_is_rejected() {
    // Arrange — 5 は SSH_AGENT_FAILURE
    let mut payload = answer_with(&[(b"blob", "x")]);
    payload[0] = 5;

    // Act & Assert
    assert_eq!(
        parse_identities(&payload),
        Err(AgentError::UnexpectedMessage { kind: 5 })
    );
}

#[test]
fn a_truncated_reply_is_reported_instead_of_panicking() {
    // Arrange
    let payload = answer_with(&[(b"blob-one", "work laptop")]);
    let cut = &payload[..8];

    // Act
    let result = parse_identities(cut);

    // Assert
    assert!(
        matches!(result, Err(AgentError::Truncated { .. })),
        "実際: {result:?}"
    );
}

#[test]
fn a_reply_claiming_more_keys_than_it_carries_is_rejected() {
    // 件数だけ大きい返事で、確保を膨らませない。
    // Arrange
    let mut payload = vec![SSH_AGENT_IDENTITIES_ANSWER];
    payload.extend_from_slice(&1_000_000u32.to_be_bytes());

    // Act
    let result = parse_identities(&payload);

    // Assert
    assert!(
        matches!(result, Err(AgentError::Truncated { .. })),
        "実際: {result:?}"
    );
}

#[test]
fn the_fingerprint_matches_the_form_ssh_add_prints() {
    // `ssh-add -l` は SHA256 を base64（パディング無し）で出す。
    // Act
    let printed = fingerprint(b"");

    // Assert — 空入力の SHA256 は既知の値
    assert_eq!(
        printed,
        "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU"
    );
}

#[test]
fn an_identity_never_carries_key_material() {
    // agent は秘密鍵を渡さない。こちらも公開鍵の生バイトすら持たない（D11）。
    // Arrange
    let identities = parse_identities(&answer_with(&[(b"SECRETBLOB", "c")])).expect("読めない");

    // Act
    let rendered = format!("{:?}", identities[0]);

    // Assert
    assert!(
        !rendered.contains("SECRETBLOB"),
        "鍵の生バイトが載っている: {rendered}"
    );
}

#[test]
fn a_comment_that_is_not_utf8_is_reported_not_silently_replaced() {
    // Arrange
    let mut payload = vec![SSH_AGENT_IDENTITIES_ANSWER];
    payload.extend_from_slice(&1u32.to_be_bytes());
    payload.extend_from_slice(&ssh_string(b"blob"));
    payload.extend_from_slice(&ssh_string(&[0xFF, 0xFE]));

    // Act & Assert
    assert_eq!(
        parse_identities(&payload),
        Err(AgentError::CommentNotUtf8 { index: 0 })
    );
}

#[test]
fn identities_are_values_that_can_be_compared() {
    let first = AgentIdentity {
        fingerprint: "SHA256:a".into(),
        comment: "x".into(),
    };
    let second = first.clone();

    assert_eq!(first, second);
}
