# プロジェクトステータス — sshboard

- **現在フェーズ**: Phase 0（未知を潰す）**着手中**
- **最終更新**: 2026-08-26

## 完了した作業

**001 はまだ「完了」ではありません。**自動で確かめられる範囲は通りました。
**残っているのは人にしかできない 2 つ**（macOS の目視・Windows の実機）です。

| 出来たもの | 中身 |
|---|---|
| `crates/sshboard-band` | 人と AI の操作が流れる 1 本の帯。`[AI]` / `[Human]` の前置、購読、**受け取り待ち（ack）**。Tauri・MCP・SSH のどれにも依存しない |
| `crates/sshboard-mcp` | アプリ同居の MCP サーバー。ツールは `ping` 1 つだけ。**帯が受け取ってから応答を返す。**127.0.0.1 の Streamable HTTP（D15） |
| `apps/desktop` | SvelteKit + Tauri 2。帯 1 本だけの画面。MCP を**同じプロセスの中で**立てる（D8） |
| `.github/workflows/ci.yml` | format / clippy / test。core は 3 OS、desktop は Windows + macOS |

### 実機で確かめた値（macOS・2026-08-26）

`curl` で生の JSON-RPC を投げ（MCP クライアント SDK を使わずに）、
`initialize → notifications/initialized → tools/call ping` を通した結果です。

| 経路 | 応答 | 所要 |
|---|---|---|
| dev（`pnpm tauri dev`） | `{"content":[{"type":"text","text":"pong"}],"isError":false}` | 0.083 秒 |
| 本番フロント（`tauri build --debug --no-bundle`） | 同上 | 0.077 秒 |

**増えたプロセスは 0 本です。**MCP は `sshboard-desktop` 自身が listen しています。

`pong` が返ること自体が帯へ出た証明になります。画面が「受け取った」と返さない限り、
ツールは応答ではなく失敗を返す作りだからです（D16）。実際、権限を入れる前は
`sshboard did not confirm the operation on screen (0/1 views acknowledged)` が返っていました。

## テスト状況

**19 本。全部通っています。**

```
cargo test --workspace                                    →  19 passed; 0 failed
cargo fmt --all -- --check                                →  差分なし
cargo clippy --workspace --all-targets -- -D warnings     →  警告なし
pnpm --filter desktop check                               →  168 files, 0 errors, 0 warnings
```

| どこ | 本数 | 何を見張っているか |
|---|---|---|
| `sshboard-band` | 9 | `[AI]` の前置・通し番号・全購読者の ack を待つ・詰まったら期限切れにする |
| `sshboard-mcp`（直接呼び） | 4 | `ping` が帯へ `[AI] ping` を載せる・**帯が受け取る前に応答を返さない**・画面が返さないときは失敗する |
| `sshboard-mcp`（HTTP 経由） | 3 | 生の JSON-RPC で `ping` が通る・**ツールが `ping` 1 つだけ**（D3 の見張り）・**loopback にしか bind していない** |
| `sshboard-desktop` | 3 | 画面が返した ack が帯へ届く・知らない行の ack を黙って捨てない・**溢れて落とした行を ack しない** |

## 未完了の作業

| # | 未知 | 状態 |
|---|---|---|
| 001 | Tauri 2 に MCP を同居させ、GUI の帯へリアルタイムに出す | **人の確認待ち**（下記 2 点） |
| 002 | 稼働中サーバーへ実際に SSH で繋ぐ | 未着手 |
| 003 | SSH ライブラリの選定（D6） | 未着手（002 の結果待ち） |
| 004 | 資格情報を OS ストア / ssh-agent から読む | 未着手 |
| 005 | `tail -f` を GUI と MCP へ同時に流す | 未着手 |

### 001 で人にしかできない残り

1. **macOS の目視。**帯に `[AI]    ping` の行が出ていることを、実物の画面で確認する
2. **Windows の実機確認。**CI で Windows のビルドと `cargo test` は回るが、
   **画面を見ての確認はできない。**手元は macOS のみ

`.claude/issues/001-mcp-in-tauri-live-band.md` の完了条件は、**AI が埋めていません**
（product-baseline §19）。

## 次のタスク

1. 001 の目視 2 件（人）
2. 通ったら 002。002 の結果で 003 が決まる

## 技術的決定

`.claude/decisions.md` を参照（D1〜D16）。

**未決は 2 つだけです。**

- **D6** — SSH ライブラリ（`russh` / `ssh2`）。Phase 0 の 003 で決める
- **D10** — 誰が実装するか

## Phase 0 で拾えた未知（001）

1. **Tauri 2 は `capabilities/` を書かないと IPC が通らない。**`listen()` は
   `plugin:event|listen` なので、権限が無いと購読ごと拒否される。
   **拒否は画面側の `catch` に落ちるだけで、Rust 側には何も出ない**
2. **本番の CSP に `connect-src ... ipc: http://ipc.localhost` が要る。**
   入れ忘れると本番ビルドでだけ IPC が死ぬ。**dev では CSP がヘッダに入らないので気づけない**
3. **描画（`requestAnimationFrame`）を ack の条件にしてはいけない。**
   WKWebView はウィンドウが前面に無いと rAF を止める。この製品は
   「人はエディタを見ていて、AI が裏で動く」使い方が普通なので、背面にした瞬間に全部失敗する

## 既知の問題

- **MCP の口に認証がありません。**loopback にしか bind していませんが、同じ端末の
  別プロセスからは叩けます。**Phase 1 で実際にサーバーを触るツールを載せる前に
  トークンを必須にすること**（D15）。Phase 0 の `ping` は何も触らないので今は無害
- **MCP のポート番号の扱いが未決。**いまは OS に空きを選ばせています。
  MCP クライアントへ登録する形が決まっていません（D15）
- **アイコンと帯の画面は仮置きです**（DESIGN.md）。人が決め直す前提
- **002 / 004 / 005 は実機のサーバーが要ります。**手元で代用できません

## 人にしかできない工程で、止まっているもの

（product-baseline §29）

- **001 の macOS 目視**と **Windows 実機確認**
- **push の最終確認**（commit は AI、push は人間）
