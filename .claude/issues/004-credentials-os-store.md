# 004. 資格情報を OS ストア / ssh-agent から読む（Phase 0）

## なぜ

**自前の鍵ストアを作らないため**（D11）。
作った瞬間に漏洩の責任を製品が引き受けます。OSS で最初に叩かれる場所でもあります。
**持たなければ守らなくてよい。**

## やること

- Windows Credential Manager から読む
- macOS Keychain から読む
- ssh-agent から鍵を使う（パスフレーズを製品が受け取らない経路）

## 完了条件

- [ ] Windows で、OS ストア経由の資格情報を使って 002 の接続が通った
- [ ] macOS で、同上
- [ ] ssh-agent 経由で、**製品がパスフレーズを一度も受け取らずに**接続が通った
- [ ] `list_connections` 相当の口が、**認証情報を含まない**ことを確認した

## やらないこと

- 独自の鍵ファイル置き場を作らない
- パスフレーズをアプリ側に保存しない

---

## dbboard から分かったこと（2026-08-26 追記）

**`keyring` 3.x に、黙って壊れる罠が記録されています（ADR-0033）。**
これを知らずに書くと、004 は「通ったように見えて通っていない」状態になります。

### 罠

`keyring = "3"` とだけ書くと、**既定の資格情報バックエンドが 1 つも入りません。**
その結果、**in-memory の mock ストアに解決されます。**

- **書き込みは `Ok` を返します。**エラーになりません
- **永続化されません。**新しい `Entry` で同じキーを読むと "no entry" が返ります
- dbboard は **Windows の初回実機**でこれを踏み、
  「Add は成功するのに Connect が `no secret stored for reference` で落ちる」形で表面化しました

### 対処（dbboard の実装をそのまま持ってくる）

target ごとに feature を明示します。**Linux の secret-service バックエンドは dbus の C
バインディングを引くので、Windows / macOS でビルドさせないよう target-scoped にします。**

```toml
[dependencies]
keyring = { workspace = true }               # どの target にも該当しない環境でコンパイルは通る（mock）

[target.'cfg(windows)'.dependencies]
keyring = { workspace = true, features = ["windows-native"] }

[target.'cfg(target_os = "macos")'.dependencies]
keyring = { workspace = true, features = ["apple-native"] }

[target.'cfg(target_os = "linux")'.dependencies]
keyring = { workspace = true, features = ["linux-native-sync-persistent", "crypto-rust"] }
```

### この罠が、テストの設計を変えます

**書いて、すぐ同じハンドルから読むテストは、mock でも通ります。**
つまり**素通りします。**

- [ ] **新しい `Entry` を作り直してから読む**テストを書く（同じハンドルを使い回さない）
- [ ] **CI の Windows / macOS の両方で**そのテストを走らせる
      （手元の OS だけで通しても、もう片方が mock に落ちていることを検出できない）

### 参考にできる既存実装

`dbboard/crates/dbboard-config/src/secrets.rs` ほか。**接続情報の保存形式（`connections.toml`
＋ OS キーチェーン）は ADR-0013。**sshboard も同じ形にすると、boardkit へ引き上げやすくなります（D5）。
