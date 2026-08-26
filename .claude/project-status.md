# プロジェクトステータス — sshboard

- **現在フェーズ**: Phase 0（未知を潰す）**着手中**
- **最終更新**: 2026-08-26

## いまの状態を一言で

**Phase 0 のうち、実機と Windows が要らない部分はすべて通しました。**
**残りは全部、人にしかできない工程です。**（product-baseline §29）

## 出来ているもの

| | 中身 |
|---|---|
| `crates/sshboard-band` | 人と AI の操作が流れる 1 本の帯。`[AI]` / `[Human]`・購読・**受け取り待ち（ack）** |
| `crates/sshboard-stream` | 1 本の出力を **GUI へは生・MCP へは素**で流す。ANSI 除去（境界の持ち越し込み）・末尾保持・停止 |
| `crates/sshboard-credentials` | OS 資格情報ストアと ssh-agent。**自前の鍵ストアを持たない**（D11） |
| `crates/sshboard-mcp` | アプリ同居の MCP。ツールは `ping` と `read_stream` の 2 つだけ |
| `apps/desktop` | SvelteKit + Tauri 2。帯 ＋ xterm.js の端末。MCP を**同じプロセスで**立てる |
| `tools/ssh-probe` | 002 / 003 用の探り棒。**製品ではない**（D6 が決まったら消す） |
| `tools/check-005.sh` | 005 の確認スクリプト |
| `.github/workflows/ci.yml` | format / clippy / test。core は 3 OS、desktop と資格情報は Windows + macOS |

## テスト状況

**58 本。全部通っています。**

```
cargo test --workspace                                 →  58 passed; 0 failed
cargo fmt --all -- --check                             →  差分なし
cargo clippy --workspace --all-targets -- -D warnings  →  警告なし
pnpm --filter desktop check                            →  170 files, 0 errors, 0 warnings
（別ワークスペース）tools/ssh-probe: cargo test        →  10 passed
```

| どこ | 本数 |
|---|---|
| `sshboard-band` | 9 |
| `sshboard-stream` | 21 |
| `sshboard-credentials` | 16 |
| `sshboard-mcp` | 9 |
| `sshboard-desktop` | 3 |
| `tools/ssh-probe`（別ワークスペース） | 10 |

## Phase 0 の 5 本

| # | 未知 | 自動で確かめた範囲 | 人にしかできない残り |
|---|---|---|---|
| 001 | MCP を同居させ帯へ出す | **通った。**dev / 本番ビルドの両方で `pong`（0.08 秒） | macOS の目視・**Windows 実機** |
| 002 | 実機 SSH | 探り棒を用意。KEXINIT 読み取りは実サーバーで確認済み | **実機に走らせる** |
| 003 | SSH ライブラリ（D6） | russh / ssh2 の両方を同じ形で試せる状態 | **002 の結果で決める** |
| 004 | 資格情報 | **通った。**mock 検出テストが macOS の実 Keychain で往復。ssh-agent へ実接続 | 実機接続・**Windows 実機** |
| 005 | GUI と MCP へ同時に流す | **通った。**同じ 1 本から GUI に ANSI 付き・MCP に素のテキスト | 実機の `tail -f`・**色の目視** |

**チェックボックスは AI が埋めていません**（product-baseline §19）。各 Issue の「実行結果」に、
私が実際に取った値だけを置いてあります。

## 次にやること（人の手が要ります）

1. **探り棒を実機に走らせる**（002 / 003）

   ```sh
   cd tools/ssh-probe
   cargo run --release -- --host <ホスト> --offer-only          # まず提示だけ見る
   cargo run --release -- --host <ホスト> --user <利用者> \
     --sftp-path /var/log --sniff /var/log/<何かのログ>
   ```

   **出力に接続先は 1 文字も出ません。そのまま貼れます。**

2. **001 / 005 の目視**（macOS）— 帯に `[AI]` の行が出ているか、端末に色が残っているか
3. **Windows 実機**（001 / 004）— CI はビルドとテストを回すが、画面は見られない
4. **push の最終確認** — commit は済んでいる。push はしていない

## 技術的決定

`.claude/decisions.md`（D1〜D17）。**未決は D6（SSH ライブラリ）と D10（実装体制）のみ。**

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

- **MCP の口に認証がありません。**loopback 限定ですが、同じ端末の別プロセスからは叩けます。
  **Phase 1 で実際にサーバーを触るツールを載せる前にトークンを必須にすること**（D15）
- **MCP のポート番号の扱いが未決。**いまは OS に空きを選ばせています（D15）
- **アイコン・帯・端末の見た目は仮置き**（DESIGN.md）。人が決め直す前提
- **`SSHBOARD_PHASE0_DEMO` と `start_demo_stream` は Phase 0 限り。**
  002 が通って本物の `tail -f` に差し替えたら消すこと
- **Windows 11 の Snap Layouts は落とす**（D17）。dbboard も解いていないので揃える
- **文字コードの扱いが未決。**EUC-JP / Shift_JIS が出るかは 002 で分かる。
  `PlainFilter` はいま U+FFFD に置換するだけで、変換していません

## 人にしかできない工程で、止まっているもの

- **実機への探り棒**（002 / 003）
- **macOS の目視**（001 / 005）と **Windows 実機**（001 / 004）
- **push の最終確認**
