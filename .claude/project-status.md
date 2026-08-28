# プロジェクトステータス — sshboard

- **現在フェーズ**: Phase 0 は技術項目を抜けた。**Phase 1 に入っています**
- **最終更新**: 2026-08-28

## いまの状態を一言で

**人も AI も、同じ 1 本の SSH でファイルを上げられます。**

Phase 0 の 5 本は、実機と Windows 目視を除いてすべて通りました。
そのうえで **D2（読み取り専用）を D22 で覆し、囲いつきの書き込みを入れています。**
運用者の実際の用途が「VPS へプロダクトを上げる」で、読み取りだけでは成立しないためです。

**残っているのは、人にしかできない工程です。**（product-baseline §29）

## 出来ているもの

| | 中身 |
|---|---|
| `crates/sshboard-band` | 人と AI の操作が流れる 1 本の帯。受け取り待ち（ack）込み |
| `crates/sshboard-stream` | 1 本の出力を GUI へは生・MCP へは素で流す。ANSI 除去・末尾保持・停止と再開 |
| `crates/sshboard-credentials` | OS 資格情報ストア ＋ ssh-agent。**自前の鍵ストアを持たない**（D11） |
| `crates/sshboard-connections` | 接続の一覧。秘密はファイルに置かず参照名だけ。印（色・タグ）付き |
| `crates/sshboard-ssh` | SSH 1 本の上の `exec` / `sftp` / `tail -f`。ホスト鍵の検証。**書き込みの囲い**（D22） |
| `crates/sshboard-engine` | **GUI と MCP が共有する実行体**（PRD §4-1）。開いている接続は 1 本だけ |
| `crates/sshboard-mcp` | 同居 MCP・**合言葉必須**（D23）。13 本のツール（下記） |
| `apps/desktop` | SvelteKit + Tauri 2。**ファイル 2 ペイン**・接続管理・帯・端末・11 言語・テーマ切替 |
| `tools/test-server` | 手元の AlmaLinux 9 sshd。`/var/log` の権限・EUC-JP のログ・色付きの成長ログを再現 |
| `tools/ssh-probe` | 002 / 003 の確認コマンド（**製品ではない**。D6 が決まったので役目は終わり） |
| `tools/check-005.sh` | 005 の確認スクリプト |
| `.github/workflows/ci.yml` | format / clippy / test |

## MCP のツール（13 本）

| 何もしない側 | サーバーへ触る側 |
|---|---|
| `ping` / `read_stream` / `session_status` | `connect` / `disconnect` |
| `list_connections` / `register_connection` / `mark_connection` | `list_directory` / `read_file` |
| | **`make_directory` / `upload_file` / `write_file`**（囲いの中だけ・D22） |

**`run_command` 相当は 1 本もありません**（D3）。
**消す・動かす・権限を変える・sudo もありません**（Phase 2 のまま）。
どちらも**ツール一覧をテストで見張っています。**

## テスト状況

**145 本。全部通っています。**（手元のテスト用サーバーを建てた状態）

```
cargo test --workspace                                 →  145 passed; 0 failed
cargo fmt --all -- --check                             →  差分なし
cargo clippy --workspace --all-targets -- -D warnings  →  警告なし
pnpm --filter desktop check                            →  198 files, 0 errors, 0 warnings
（別ワークスペース）tools/ssh-probe: cargo test        →  10 passed
```

| どこ | 本数 |
|---|---|
| `sshboard-band` | 9 |
| `sshboard-stream` | 24 |
| `sshboard-credentials` | 16 |
| `sshboard-connections` | 20 |
| `sshboard-ssh` | 36（うち実機 13） |
| `sshboard-engine` | 9（うち実機 7） |
| `sshboard-mcp` | 27（うち実機の通し 4） |
| `sshboard-desktop` | 4 |
| `tools/ssh-probe`（別ワークスペース） | 10 |

**実機の通し 4 本**が要です。MCP（HTTP・合言葉つき）→ Engine → SSH 1 本 → sftp を、
**外部クライアントと同じ生の JSON-RPC** で叩き、囲いの外を断ったあと
**サーバー側の一覧を見て本当に届いていないこと**まで確かめています。

## Phase 0 の 5 本

| # | 未知 | 自動で確かめた範囲 | 人にしかできない残り |
|---|---|---|---|
| 001 | MCP を同居させ帯へ出す | **通った**（dev / 本番ビルド） | macOS 目視・**Windows 実機** |
| 002 | 実機 SSH | **通った。**9 台・両ライブラリ・全項目 OK | — |
| 003 | SSH ライブラリ（D6） | **決定 — `russh` + `russh-sftp`** | — |
| 004 | 資格情報 | **通った。**mock 検出が実 Keychain で往復・ssh-agent へ実接続・接続管理も動く | **Windows 実機** |
| 005 | GUI と MCP へ同時に流す | **通った。**実機の `tail -f` で、GUI は色付き・MCP は素 | 色の目視 |

**チェックボックスは AI が埋めていません**（product-baseline §19）。

## 次にやること

1. **実運用で 1 回上げてみる**（人）。**ここが本当の合否です。**
   接続を登録し、`write_roots` を決め、ファイルの面から上げる。
   そのあと MCP から同じことを AI にやらせる
2. **ダウンロード**（サーバー → 手元）。いまは上げる側しかない
3. **端末タブ**。ファイルの面はできたが、Tera Term 側の面がまだ
4. **`run_readonly`**（許可リスト方式）。`df` / `ps` / `systemctl status` 相当
5. **Windows 実機**（001 / 004）。CI はビルドとテストを回すが、画面は見られない
6. **push の最終確認**（人）

## 技術的決定

`.claude/decisions.md`（D1〜D24）＋ `DESIGN.md`。**未決は D10（実装体制）のみ。**

## Phase 0 で拾えた未知

1. **Tauri 2 は `capabilities/` を書かないと IPC が通らない。**拒否は画面側の `catch` に
   落ちるだけで、Rust 側には何も出ない
2. **本番の CSP に `connect-src ... ipc: http://ipc.localhost` が要る。**
   dev では CSP がヘッダに入らないので、入れ忘れに気づけない
3. **描画（`requestAnimationFrame`）を ack の条件にしてはいけない。**
   WKWebView は背面のウィンドウで rAF を止めるため、背面にした瞬間に MCP が全部失敗する
4. **`keyring` 3.x は既定バックエンドが無いと mock に落ちる**（dbboard ADR-0033）。
   書き込みは `Ok` を返すのに永続化されない。**テストの設計まで変わる**
5. **`russh` の既定は `aws-lc-rs` を引く。**dbboard の ADR-0034 と食い違うので
   `default-features = false` ＋ `ring` へ寄せる
6. **`russh` に SFTP は無い**（別 crate）。`ssh2` は内蔵。**D6 はここで割れる**

## 既知の問題

- **MCP のポート番号の扱いが未決。**いまは OS に空きを選ばせています（D15）。
  **起動するたびにポートが変わるので、`claude mcp add` をやり直すことになります。**
  合言葉は固定しましたが、URL はまだ固定していません
- **大きいファイルは丸ごとメモリに載ります**（上限 8 MB / MCP）。
  分割送信は、実測でそこに当たってから入れます
- **ダウンロードがありません。**上げる側だけです
- **`PerSourcePenalties` に当たる可能性**（D24）。製品側の連打防止はまだ入っていません
- **アイコン・帯・端末の見た目は仮置き**（DESIGN.md）。人が決め直す前提
- **`SSHBOARD_PHASE0_DEMO` と `start_demo_stream` は Phase 0 限り。**
  002 が通って本物の `tail -f` に差し替えたら消すこと
- **Windows 11 の Snap Layouts は落とす**（D17）。dbboard も解いていないので揃える
- **文字コードを変換していません。**EUC-JP のログは実在します（実機・手元のサーバーで再現済み）。
  読み出しは U+FFFD へ置換したうえで「置換した」と伝えるだけで、変換はしていません

## Phase 1 で拾えた未知

7. **OpenSSH 9.8+ の `PerSourcePenalties` は、ホスト鍵の検証そのものを罰する**（D24）。
   「繋いで、鍵を見て、信用できなければ認証せず切る」が罰の対象。**実際にテストが落ちた**
8. **macOS のキーチェーン ACL はバイナリに紐づく**（D23）。未署名の開発ビルドは
   ビルドのたびに別の相手になるので、「常に許可」を押しても**毎回承認が出る**
9. **rmcp の引数名は Rust のフィールド名そのまま**（snake_case）。camelCase では通らない
10. **`serve()` と `Engine` が別々に接続一覧の場所を持てた。**
    `list_connections` が見ている一覧と `connect` が引く一覧が食い違いうる状態だった。
    **テストで実際に食い違って気づいた**

## 人にしかできない工程で、止まっているもの

- **実運用で 1 回上げること。**ここが通らなければ、上の 145 本は意味を持ちません
- **macOS の目視**（001 / 005）と **Windows 実機**（001 / 004）
- **push の最終確認**
