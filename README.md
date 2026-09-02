# sshboard

**MCP SFTP client and MCP SSH terminal — let an AI agent *see* your remote server, over one SSH session, on the same screen you are looking at.**

リモートサーバーの中身を、**人と AI が同じ画面で見る**ための道具です。
ファイルも、コマンドの出力も、**同じ 1 本の SSH の上**で見ます。

> ⚠️ **α です。実装はありますが、まだ一度も実運用で起動されていません。**
> テストは 342 本通っており（Rust 267 / フロント 75）、Windows・macOS の
> インストーラも束ねられています。**それと道具として使えることは別です。**
> 方向性は [`PRD.md`](PRD.md)、進め方は [`.claude/roadmap.md`](.claude/roadmap.md)、
> 決定と理由は [`.claude/decisions.md`](.claude/decisions.md) にあります。

---

## なぜ作るか

従来型のレンタルサーバー / VPS の上で動き続けているサービスがあります。
そこを AI と一緒に保守すると、毎回これが起きます。

```
人 : このサービスの設定を直したい
AI : サーバー側の状態が分からないので教えてください
人 : （ターミナルを開いて調べて、貼る）
AI : ではこの設定ファイルの中身も見せてください
人 : （SFTP を開いて落として、貼る）
```

**この往復を消すための道具です。**
AI にサーバーを操作させる必要はありません。**見せるだけで消えます。**

## どう使うものか

**画面のボタンを押す道具ではありません。**あなたはターミナルの AI エージェントに話しかけ、
エージェントが sshboard の内蔵 MCP を呼びます。

```
あなた → ターミナルの AI エージェントへ「メールが届かない。調べて」
              │
              │  MCP（アプリ内蔵・別プロセスを立てない）
              ▼
        sshboard  ── SSH 1 本 ──▶  サーバー
              │      （sftp / exec）
              ▼
   sshboard の画面に [AI] の行が流れる
   ── あなたは見ているだけ。何を読んで何を打ったかが、その場で分かる
```

```
[Human] $ cd /var/www
[AI]    $ df -h
Filesystem      Size  Used Avail Use% Mounted on
/dev/vda1        50G   38G   10G  80% /
[AI]    read /etc/postfix/main.cf  (4.1 KB)
```

**人の側は制限しません。**ファイル 2 ペインとターミナルタブは、普通の SFTP クライアント /
ターミナルとして自由に使えます。**制限するのは AI の口だけ**です。

### AI が呼べるもの（Phase 1・読み取りのみ）

`list_connections` / `list_directory` / `stat` / `read_file` / `search` / `download` /
`disk_usage` / `process_list` / `service_status` / `runtime_versions` / `read_log` /
`network_listen` / `run_readonly`（許可リスト方式）

**`run_command(cmd)` を作りません。**1 つ置けば全部できてしまい、破壊的操作を防ぐ手段が
「使わない約束」しか残らないからです。**約束は手順書であって、ゲートではありません。**

`run_readonly` で AI が渡せるのは、**人が `readonly.toml` に書いた項目の識別子だけ**です。
引数のスロットはありません。**既定は空 ＝ 書くまで 1 本も走りません。**
断った分は画面の帯に出て、`readonly-refused.log` に残ります。**足すのは人です。**

## 成功条件（唯一）

> **AI が「サーバー側の状態を教えてください」と言わなくなること。**

ダウンロード数でも star 数でもありません。実際の保守作業で往復が消えたかどうかです。

## 用途を近代化しません

- サーバーを別の基盤へ移させる道具ではありません
- **従来の構成のまま使えること**が価値です
- 従来型のサービスは今後 10 年以上残ります。そこに AI 開発を持ち込む道具が存在していません

## 形

```
   ┌──────────┬──────────┐        ┌──────────┐
   │ ファイル  │  端末     │        │   MCP    │
   │  2 ペイン │  タブ     │        │          │
   └────┬─────┴────┬─────┘        └────┬─────┘
        └──────────┴───────┬───────────┘
                           ▼
              ┌────────────────────────┐
              │  Operation Engine      │  ← 実装はここに 1 つだけ
              └───────────┬────────────┘
                          ▼
                       SSH 1 本
                    （sftp / exec）
```

**SFTP の実装を 2 つ持ちません。裏で見えない SSH セッションを張りません。**
見えないことが最大の危険だからです。

誰が触ったかは同じ帯に流れます。**面が違っても記録は 1 本です。**

```
[Human] $ cd /var/www
[AI]    $ df -h
Filesystem      Size  Used Avail Use% Mounted on
/dev/vda1        50G   38G   10G  80% /
[AI]    read /etc/postfix/main.cf  (4.1 KB)
```

## Phase 1 は読み取り専用

**書き込みを一切入れません。**用途（障害調査・メール設定調査・提案）がすべて読み取りだからです。
危険がほぼゼロになるので、**稼働中の本番サーバーを初日から対象にできます。**

AI が呼べるもの:
`list_connections` / `list_directory` / `stat` / `read_file` / `search` / `download` /
`disk_usage` / `process_list` / `service_status` / `runtime_versions` /
`read_log` / `network_listen` / `run_readonly`（許可リスト方式）

### `run_command(cmd)` を作りません

任意コマンドを 1 つ置けば全部できます。**だから作りません。**

- 渡した時点で、破壊的操作を防ぐ手段が「使わない約束」しか残らなくなります
- **約束は手順書であって、ゲートではありません**
- 許可リスト方式なら、**危険なコマンドは呼びようがありません**

#### `readonly.toml` の形（人が書きます）

```toml
version = 1

[[command]]
id = "uptime"          # AI が渡せる唯一の値
run = "uptime"         # 実際に走る文字列。**人が書いたものがそのまま走ります**
description = "稼働時間"
```

**製品は既定の項目を 1 本も持ちません。**実務で何が要るかの一覧を誰も持っていないので、
推測で埋めると必ず外します。AI が呼んで断られた分が `readonly-refused.log` に溜まるので、
**そこを見て、本当に要ったものだけを足してください。**

**この仕組みが検証できないこと:** 書いた `run` が本当に読み取り専用かどうかは、
**製品には分かりません。**`uptime` と `rm -rf /` を機械が見分ける方法はありません。
ここが防ぐのは「AI がコマンド文字列を組み立てること」だけです。

**人（GUI）の側は制限しません。**普通の SFTP クライアント / ターミナルとして自由に使えます。
制限するのは AI の口だけです。

## やらないこと

| やらない | 理由 |
|---|---|
| AI に書き込みを渡す（Phase 1） | 用途がすべて読み取り。本番サーバーを初日から対象にできる |
| AI に任意コマンドを渡す | 許可リストで構造的に防ぐ |
| AI に sudo を渡す | Phase 1 で権限昇格を扱わない |
| 自前の鍵ストアを作る | OS 資格情報ストア / ssh-agent へ委譲する。持たなければ守らなくてよい |
| AI チャット UI を内蔵する | エージェントはアプリの外にいる |
| レポート生成機能を作る | 読めれば提案は AI が書く。足すものが無い |
| サーバーの移行を促す | 従来構成のまま使えることが価値 |

## 動かす

**まだ α です。**実運用のサーバーへ向ける前に、**手元のテスト用サーバーで一度動かしてください。**

### 前提

| | |
|---|---|
| OS | macOS / Windows（Linux は配布対象外） |
| Rust | `rust-toolchain.toml` が 1.98.0 を指定。`rustup` が入っていれば自動で揃います |
| Node | 20 以上 |
| pnpm | `corepack enable`（**npm / yarn は使いません**） |
| Docker | 手元のテスト用サーバーを建てる場合だけ |

**鍵は ssh-agent に入れておくのを勧めます。**そうすれば sshboard はパスフレーズを
一度も受け取りません。

### 起動

```sh
pnpm install
pnpm --filter desktop tauri dev
```

### テスト

```sh
cargo test --workspace         # Rust
pnpm --filter desktop test     # フロント
pnpm --filter desktop check    # 型検査
```

### 手元のテスト用サーバー

**あなたのサーバーには一切触りません。**使い捨ての鍵を作り、Docker で 1 台建てます。

```sh
sh tools/test-server/up.sh        # 建てる
sh tools/test-server/up.sh down   # 片付ける
```

### AI（MCP）から繋ぐ

起動すると、画面に MCP の URL と合言葉が出ます。それを `claude mcp add` へ渡します。

> **ポートは起動するたびに変わります**（未決の課題）。
> **いまは、立ち上げ直すたびに登録し直しになります。**

### α で知っておいてほしいこと

- **署名していません。**macOS は Gatekeeper が、Windows は SmartScreen が止めます
- **AI が書けるのは、接続ごとに人が列挙したディレクトリの下だけ**です。**既定は空 ＝ 1 バイトも書けません**
- **`run_readonly` の許可リストも既定は空**です。`readonly.toml` に人が書くまで 1 本も走りません。
  用途別のツール（`disk_usage` など）は、書かなくても動きます
- **端末は人と AI で共有します。**AI が握っている間は人の入力が締まり、
  **［止める］はいつでも効きます**
- **インストーラは Release に付けます**（D32）。
  Windows は `.msi` と NSIS の `-setup.exe`、macOS は `.app.zip`。
  **未署名なので、Windows は SmartScreen、macOS は Gatekeeper が止めます。**
  **警告が出て分からなかった、は Issue に書いてください** — 署名を買う判断の材料です（D12）

## 技術スタック

| | |
|---|---|
| 殻 | Tauri 2（Windows / macOS） |
| 本体 | Rust |
| 画面 | SvelteKit + xterm.js（**ANSI の解釈を自前で書かない**） |
| SSH | **`russh` + `russh-sftp`**（9 台の実機で両方試して決定・D6） |
| MCP | **アプリ内蔵**（別バイナリにしない・別ビルドを要求しない） |
| 資格情報 | OS 資格情報ストア + ssh-agent |

## ライセンス

MIT

## 開発ルール

このリポジトリは AI エージェント開発のベースルールに従います。
詳細は [`.claude/rules/product-baseline.md`](.claude/rules/product-baseline.md) と [`CLAUDE.md`](CLAUDE.md) を参照してください。

- commit は AI、push は人間（人間確認なしの push 禁止）
- テスト後回し・削除禁止
- **public リポジトリです。**接続先ホスト名 / IP / ユーザー名・個人名・個人メールを
  コード・文書・commit history・スクリーンショットに残さないこと
