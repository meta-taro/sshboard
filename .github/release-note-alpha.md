## これは α です

**まだ一度も実運用で起動されていません。** テストは通っていますが、
それと道具として使えることは別です。

### ⚠️ 署名していません。開くときに警告が出ます

| | 何が出るか | どうするか |
|---|---|---|
| Windows | SmartScreen が止める | 「詳細情報」→「実行」 |
| macOS | Gatekeeper が止める | 右クリック →「開く」 |

**これは鍵を扱う道具です。**「警告を無視して開く」を最初に教えることになるのは
承知のうえで出しています。**署名は、未署名で困る人が実際に出てから入れます**（D12）。
**警告が出た・分からなかった、は Issue に書いてください。**それが買う判断の材料です。

zip から出した直後の Windows ファイルには Mark of the Web が付きます。
右クリック → プロパティ → **「許可する」にチェック**しておくと静かになります。

### 添付しているもの

| ファイル | 何 |
|---|---|
| `sshboard_0.1.0_x64-setup.exe` | Windows インストーラ（NSIS）。**こちらが素直です** |
| `sshboard_0.1.0_x64_en-US.msi` | Windows インストーラ（MSI） |
| `sshboard_0.1.0_macos.app.zip` | macOS。展開して `/Applications` へ |

Windows は **WebView2** が要ります。Windows 11 と最近の 10 には入っています。
無ければインストーラが取りに行くので、**初回はインターネット接続が必要**です。

接続設定の置き場所:

- Windows: `%APPDATA%\sshboard\sshboard\config\connections.toml`
- macOS: `~/Library/Application Support/dev.sshboard.sshboard/connections.toml`

### ソースから建てる場合

**実運用のサーバーへ向ける前に、手元のテスト用サーバーで一度動かしてください。**

```sh
pnpm install
sh tools/test-server/up.sh
pnpm --filter desktop tauri dev
```

### 承知しておいてほしいこと

- **MCP のポートは起動するたびに変わります。** 立ち上げ直すたびに `claude mcp add` をやり直すことになります
- **AI が書けるのは、接続ごとに人が列挙したディレクトリの下だけ**です。**既定は空 ＝ 1 バイトも書けません**
- **`run_readonly` の許可リストも既定は空**です。`readonly.toml` に人が書くまで 1 本も走りません
- **端末は人と AI で共有します。** AI が握っている間は人の入力が締まり、**［止める］はいつでも効きます**
- **Windows は実機で確認していません。** CI でビルドは通していますが、
  ssh-agent（名前付きパイプ / Pageant）に実際に繋がるかは未確認です

### 不具合の出し方

**使った人が、使った直後に、自分の言葉で** Issue へ書いてください。
要約された時点で、本当に困っていた所が落ちます。
