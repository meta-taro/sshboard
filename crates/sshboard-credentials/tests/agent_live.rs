//! 動いている ssh-agent に対するテスト。
//!
//! **agent が無い環境でも走ります**（product-baseline §4）。
//! 無いときは「無い」と正しく答えることを見ます。

use sshboard_credentials::{list_identities, AgentConnectError};

#[test]
fn asking_the_agent_either_lists_keys_or_says_it_is_not_running() {
    // Act
    let result = list_identities();

    // Assert
    match result {
        Ok(identities) => {
            // 鍵が 0 本でも異常ではない。**形だけ確かめる。**
            for identity in &identities {
                assert!(
                    identity.fingerprint.starts_with("SHA256:"),
                    "指紋の形が違います: {identity:?}"
                );
            }
            println!("ssh-agent は {} 本の鍵を持っています", identities.len());
        }
        Err(AgentConnectError::NotRunning { .. }) => {
            println!("ssh-agent は動いていません（この環境では想定内）");
        }
        Err(other) => panic!("agent が動いているのに読めません: {other}"),
    }
}

#[test]
fn an_agent_error_never_carries_key_material() {
    // Arrange & Act
    let rendered = match list_identities() {
        Ok(identities) => format!("{identities:?}"),
        Err(error) => format!("{error}"),
    };

    // Assert — 秘密鍵の PEM 見出しが混ざっていない
    assert!(!rendered.contains("PRIVATE KEY"), "秘密鍵が混ざっています");
}
