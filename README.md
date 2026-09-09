# sshboard

**MCP SFTP client and MCP SSH terminal — let an AI agent *see* your remote server, over one SSH session, on the same screen you are looking at.**

リモートサーバーの中身を、**人と AI が同じ画面で見る**ための道具です。
ファイルも、コマンドの出力も、**同じ 1 本の SSH の上**で見ます。

> ⚠️ **α です。**実運用のサーバーで**使われ始めました**（2026-09-09・証明書更新の調査）。
> テストは **550 本**通っており（Rust 341 / フロント 209）、Windows・macOS の
> インストーラも束ねられています。**それと道具として使えることは別です** ——
> 実際、**端末に 1 バイトも出ていない状態を 3 日間配っていました**（Issue #10）。
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

## 状態を変える操作（`operations.toml`）

> **既定は空です。**このファイルを書くまで、AI は 1 本も走らせられません。

読み取りの許可リスト（`readonly.toml`）とは**別のファイル**です。
**`readonly` という名前のファイルに書き込み操作を入れない**ためで、
名前で区別がつくことが要点です。`connections.toml` と同じディレクトリに置きます。

```toml
version = 1

[[operation]]
id = "restart-httpd"
run = "sudo systemctl restart httpd"
description = "Apache を再起動する"
max_per_hour = 3
```

| 項目 | 何を書くか |
|---|---|
| `id` | **AI が渡せるのはこれだけ。**短く、読んで分かる名前 |
| `run` | **実際に打つもの。**AI は組み立てられません |
| `description` | **承認の画面に出ます。**半年後の自分が読んで分かる言葉で |
| `max_per_hour` | 1 時間あたりの上限。**0 は書けません**（止まる所が要るため） |

### 走るまでに通る関門

1. **人が書いた一覧に在ること。**無い id は断られ、断った事実が記録に残ります
2. **1 時間あたりの上限を超えていないこと**
3. **人が画面で許可すること。**AI からの 1 回目は必ず断られ、
   画面に「**何が走るのか**」がそのまま出ます
4. **許可は使い切り。**1 回の許可で 1 回だけ走ります

**人（画面）は 3 と 4 を通りません。**画面の前に居るなら、見えているからです。

### 書いても弾かれるもの

**設定で有効にできません。**`operations.toml` に書いても断ります。

- 檻そのもの —— `sudoers` / `visudo` / `authorized_keys`
- **鍵** —— `.ssh` / `id_rsa` / `id_ed25519` / `id_ecdsa`
- **パスワードの入れ物** —— `/etc/shadow` / `/etc/gshadow`
- sshboard 自身の設定 —— `connections.toml` / `readonly.toml` / `operations.toml`
- 利用者とロール —— `useradd` / `usermod` / `userdel` / `groupadd`
- 壊す操作 —— `mkfs` / `fdisk` / `dd ` / `rm -rf`

**AI が自分の檻を広げられないこと**が、この一覧の芯です。

## root が要る操作（`become`）

`operations.toml` は「**どのコマンドを走らせてよいか**」を決めます。
「**どうやって権限を得るか**」は、**接続ごとに書きます**（`connections.toml`）。

```toml
[[connections]]
id = "..."
become = "sudoers"   # サーバー側で範囲が切ってある（推奨）
# become = "ask"     # 要るときに人へ聞く。**保存しません**
# 書かなければ「上げない」
```

| `become` | sshboard が打つもの | パスワード |
|---|---|---|
| 書かない | 人が `run` に書いたまま | 要りません |
| `sudoers` | `sudo -n …` | 要りません（`sudoers.d` が持つ） |
| `ask` | `sudo -S -p '' …` | **人が画面でその場で入れます** |

**`sudo ` で始まる `run` だけが対象**です。それ以外は 1 文字も変わりません。
`ask` にしても、`sudo` を使わない操作でパスワードを聞かれることはありません。

### `sudoers`（推奨）

```
<利用者> ALL=(root) NOPASSWD: /usr/bin/systemctl restart httpd, /usr/bin/systemctl reload httpd
```

```sh
# 必ず visudo で。書き間違えると、誰も sudo できなくなります
sudo visudo -f /etc/sudoers.d/sshboard
sudo chmod 0440 /etc/sudoers.d/sshboard
```

**これは OS が強制するゲート**で、アプリの約束ではありません。
範囲が `sudoers` に書いてあるので**後から人が読んで検証でき**、
**sshboard 側に秘密が 1 つも増えません。**

効いているかは、こう確かめます。

```sh
sudo -n systemctl reload httpd    # パスワードを聞かれずに通れば OK
sudo -n systemctl restart nginx   # 書いていないものは断られるはず
```

**2 つ目が通ってしまったら、範囲が広すぎます。**

### `ask`（`sudoers` を触れないとき）

**サーバーを触れない事情は実在します** —— 借りている・権限が無い・台数が多い。
`sudo -n` が「パスワードが必要です」と返るサーバーでは、`sudoers` の道は
**最初から閉じています。**そのための `ask` です。

承認の画面にパスワードの欄が出て、**人がその場で入れます。**

- **保存しません。**ディスクにも OS ストアにも置きません
- **コマンド行に載せません。**標準入力から渡すので、`ps` に出ません
- **画面にも記録にも出ません。**出るのは `$ sudo -S -p '' …` までです
- **1 回の許可で 1 回だけ。**走った瞬間に捨てます
- **断れば、その場で捨てます**

**root しか読めないログを読む**のも、この形です。

```toml
[[operation]]
id = "read-letsencrypt-log"
run = "sudo tail -n 200 /var/log/letsencrypt/letsencrypt.log"
description = "証明書の更新がなぜ失敗したかを見る"
max_per_hour = 6
```

**読み取りの許可リスト（`readonly.toml`）には効きません。**
効かせると、**読み取りのたびにパスワードを聞かれます。**
root が要る読み取りは、`operations.toml` に書いてください。

`su` で先に root になる運用は**勧めません** —— **範囲が消えます。**
`su -` しか無いサーバー向けの道は、**まだありません**（`.claude/decisions.md` D48）。

### 製品が保証できないこと

**「本当にその操作だけをするか」を、製品は検証できません。**
走るのは人が書いた文字列です。`readonly.toml` と同じで、**そこは人が読んで
確かめる所**です。だから `sudoers` で **OS に強制させます。**

**出先への承認の届け先は、まだありません**（`.claude/decisions.md` の D47・未決）。
いまは**画面の前に居る人だけ**が答えられます。人が居なければ操作は走りません。

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

#### すすめる形 — 合言葉をどこにも書かない

```sh
# macOS
claude mcp add sshboard -- /Applications/sshboard.app/Contents/MacOS/sshboard --mcp-stdio-proxy

# Windows
claude mcp add sshboard -- "%LOCALAPPDATA%\\sshboard\\sshboard.exe" --mcp-stdio-proxy
```

**この形なら、合言葉はどこにも残りません**（Issue #2）。
`--mcp-stdio-proxy` で起動された sshboard は、**窓を作らず中継として走ります。**
合言葉は自分で読み、動いている本体へ流すだけです。

- **`~/.claude.json` にもコマンド履歴にも合言葉が載りません**
- ポートを移しても（`SSHBOARD_MCP_PORT`）**登録し直しは要りません**
- 本体が動いていなければ「**sshboard が動いていません。アプリを起動してから、
  もう一度お試しください。**」と返ります

**SSH を張るのは本体だけです。**中継は Engine も帯も持ちません。
**見えない SSH セッションは 1 本も増えません**（禁止事項 3）。

#### 直に HTTP で繋ぐ形

画面右上の **「MCP の登録コマンドを写す」ボタン**から、合言葉ごと 1 行を写せます。

```sh
claude mcp add --transport http sshboard http://127.0.0.1:22022/mcp \
  --header "Authorization: Bearer <合言葉>"
```

**`--transport http` が要ります。**パスは `/mcp` で、合言葉は `Authorization: Bearer` で渡ります。

**ポートは `22022` に固定です**（D33）。合言葉も使い回すので、
**登録は 1 回で終わります。**立ち上げ直しても同じ URL のままです。

> ⚠️ **`claude mcp add` を使うと、合言葉が `~/.claude.json` に平文で残ります。**
> loopback の取っ手なので鍵そのものではありませんが、
> **これを持っている相手は、あなたの繋いでいるサーバーを読めます。**
> 消すときは `claude mcp remove sshboard` を忘れずに。

`.mcp.json` に書いても構いません（**こちらもファイルに平文で残ります**）。

```json
{
  "mcpServers": {
    "sshboard": {
      "type": "http",
      "url": "http://127.0.0.1:22022/mcp",
      "headers": { "Authorization": "Bearer <画面の「MCP」ボタンから写せます>" }
    }
  }
}
```

> ポートがぶつかったら、**黙って別の番号へ逃げません。**
> 画面に「立ち上がりませんでした（ポート 22022）」と出します。
> 移すときは環境変数 `SSHBOARD_MCP_PORT` を設定してください。

### 更新

**新しい版が出ると、起動したときに画面へ出ます**（D34）。

**黙って入れ替えません。**落として入れるのも、再起動するのも、押すのは人です。
この道具は SSH の鍵を扱うので、無断で自分を書き換える形にはしていません。

更新そのものは **minisign で署名されており、Tauri が必ず検証します。**
**コード署名（D12・OS の警告が出るかどうか）とは別の話**で、
「配布元が本物か」はこちらで担保されています。

### α で知っておいてほしいこと

- **署名していません。**ただし **Windows で止まるかどうかは、取り方で変わります**（Issue #3・実測）
  - **ブラウザで取ってダブルクリック** → SmartScreen が止まります
  - **`gh run download` / CI / スクリプトで取る** → **止まりません。**
    Mark of the Web が付かず、SmartScreen のアプリ評価チェックが起点を失うため
  - macOS の Gatekeeper は未検証です
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
