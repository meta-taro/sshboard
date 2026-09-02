//! 書き出す／取り込むときの段取り（D18）。
//!
//! **暗号そのものはここに無い**（`lib.rs`）。ここが決めるのは
//! **秘密をどこから集め、どこへ戻し、途中で失敗したらどうするか**です。
//!
//! 保管庫（OS の資格情報ストア）は [`SecretVault`] で抽象しています。
//! **実物の Keychain を触らずに段取りを試せるようにする**ためで、
//! `keyring` が mock に落ちていても気づけない罠（dbboard ADR-0033）を
//! 段取りの試験に持ち込まないためでもあります。

use std::collections::BTreeMap;

use sshboard_connections::Connections;

use crate::BundlePayload;

/// 秘密の置き場所。**製品では OS の資格情報ストア**（D11）。
pub trait SecretVault {
    /// # Errors
    /// 取り出せないとき。
    fn get(&self, reference: &str) -> Result<String, String>;
    /// # Errors
    /// 入れられないとき。
    fn put(&self, reference: &str, secret: &str) -> Result<(), String>;
    /// # Errors
    /// 消せないとき。
    fn delete(&self, reference: &str) -> Result<(), String>;
}

/// 段取りの失敗。**「暗号が失敗した」とは別に扱います。**
#[derive(Debug)]
pub enum TransferError {
    /// チェックされた識別子が一覧に無い。
    UnknownConnection { id: String },
    /// 接続が指している秘密を、保管庫から取り出せない。
    SecretMissing { reference: String, detail: String },
    /// 保管庫へ入れられない。
    SecretStore { reference: String, detail: String },
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownConnection { id } => write!(f, "そんな接続はありません: {id}"),
            Self::SecretMissing { reference, detail } => write!(
                f,
                "`{reference}` の秘密を取り出せません（{detail}）。\
                 パスフレーズを登録し直すか、その接続を外してください"
            ),
            Self::SecretStore { reference, detail } => {
                write!(f, "`{reference}` を保存できません（{detail}）")
            }
        }
    }
}

impl std::error::Error for TransferError {}

/// チェックされた接続だけを集めて、暗号化する前の中身を組む。
///
/// **チェックしていない接続の秘密は持っていきません。**
///
/// # Errors
///
/// 識別子が見つからない、または秘密を取り出せない場合。
pub fn build_payload(
    all: &Connections,
    ids: &[String],
    vault: &dyn SecretVault,
) -> Result<BundlePayload, TransferError> {
    let mut chosen = Vec::with_capacity(ids.len());
    let mut secrets = BTreeMap::new();

    for id in ids {
        let entry = all
            .connections
            .iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| TransferError::UnknownConnection { id: id.clone() })?;

        if let Some(reference) = &entry.keyring_passphrase_ref {
            // **黙って穴の開いたファイルを作らない。**
            // 相手は「入っているはず」で受け取り、繋げないところで初めて気づきます。
            let secret = vault
                .get(reference)
                .map_err(|detail| TransferError::SecretMissing {
                    reference: reference.clone(),
                    detail,
                })?;
            secrets.insert(reference.clone(), secret);
        }
        // **鍵のパスフレーズを持たない接続は、それでよい**（D18）。
        // ssh-agent へ預けているだけで、欠けているわけではありません。

        chosen.push(entry.clone());
    }

    Ok(BundlePayload::new(
        Connections {
            version: all.version,
            connections: chosen,
        },
        secrets,
    ))
}

/// 取り込んだ中身を、いまの一覧へ混ぜる。
///
/// **秘密を保管庫へ入れてから、一覧を返します**（呼び出し側が保存する）。
/// 順番が逆だと、**一覧には載っているのに秘密が無い接続**ができます。
///
/// 途中で入れられなくなったら、**それまでに入れた分を戻してから**返します。
/// 半分だけ入った状態は、あとから見て何が起きたか分かりません。
///
/// # Errors
///
/// 保管庫へ入れられない場合。
pub fn apply_payload(
    mut payload: BundlePayload,
    current: Connections,
    vault: &dyn SecretVault,
) -> Result<Connections, TransferError> {
    let secrets = std::mem::take(&mut payload.secrets);
    let mut put: Vec<String> = Vec::with_capacity(secrets.len());

    for (reference, secret) in &secrets {
        if let Err(detail) = vault.put(reference, secret) {
            // **入れた分を戻す。**戻せなかったものは、これ以上できることが無いので進めます
            // （元の失敗の方を人へ返す）。
            for done in &put {
                let _ = vault.delete(done);
            }
            return Err(TransferError::SecretStore {
                reference: reference.clone(),
                detail,
            });
        }
        put.push(reference.clone());
    }

    // `Connections` に `Default` は無いので、空を作って入れ替えます。
    // **`Drop` を持つ型は分解して取り出せない**ため、置き換えで取ります。
    let incoming = std::mem::replace(
        &mut payload.connections,
        Connections {
            version: 0,
            connections: Vec::new(),
        },
    );

    // 同じ識別子は**入ってきた方で置き換える。**元からあって被らないものは残す。
    let mut merged: Vec<_> = current
        .connections
        .into_iter()
        .filter(|existing| !incoming.connections.iter().any(|e| e.id == existing.id))
        .collect();
    merged.extend(incoming.connections);

    Ok(Connections {
        version: current.version,
        connections: merged,
    })
}
