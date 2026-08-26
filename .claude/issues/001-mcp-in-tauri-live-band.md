# 001. Tauri 2 に MCP を同居させ、呼び出しを GUI の帯へ出す（Phase 0・**最優先**）

**ここが成立しないなら、この製品は成立しません。**最初にここを見ます。

## なぜ最初か

この製品の価値は「AI がサーバーを操作できること」ではなく、
**AI が読んだもの・打ったものが人の画面にその場で流れること**です（PRD §4-2）。

MCP を別プロセスにすると、GUI は別の経路で同じ SSH を張ることになり、
**裏で見えないセッションが 1 本増えます。**それは危険そのものなので、
同居できないなら設計を作り直すか、製品を取り下げます。

## やること

- Tauri 2 のアプリを 1 つ作る（画面は帯 1 本だけでよい。ファイル 2 ペインも端末もまだ要らない）
- **そのプロセスの中で** MCP サーバーを立てる（別バイナリにしない・D8）
- MCP ツールを 1 つだけ実装する。中身はダミーでよい（例: 固定文字列を返す `ping`）
- 外部の AI クライアントから MCP を叩き、**GUI の帯に行が増える**ところまで

## 完了条件

- [ ] 別プロセスを立てずに、MCP ツールの呼び出しが GUI の帯へ 1 行として出る
- [ ] 行頭に `[AI]` が付く（人の操作と区別できる）
- [ ] 帯への反映がツール応答より先か同時（**AI が返答したあとに画面が追いつく形にしない**）
- [ ] Windows と macOS の両方で確認した

## やらないこと

- SSH に繋がない（002 でやる）
- 認証情報を扱わない（004 でやる）
- 画面を作り込まない。**帯が 1 本出れば足りる**

---

## 実行結果（2026-08-26・macOS のみ）

**チェックボックスは埋めていません。**合否は実物の画面を見た人が付けてください（product-baseline §19）。
ここには、私が実際に取った値だけを置きます。

### 何を動かしたか

`curl` で生の JSON-RPC を投げています。**MCP クライアント SDK を使っていません。**
SDK 同士で話せても、別実装のクライアントで動く保証にならないためです。

```
initialize → notifications/initialized → tools/call ping
```

### 出た値

**dev（`pnpm tauri dev`・画面は vite の dev サーバー）**

```
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}],"isError":false}}
所要 0.083 秒 / 4 回とも同じ
```

**本番フロント（`pnpm tauri build --debug --no-bundle` で作った実行ファイル）**

```
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}],"isError":false}}
所要 0.077 秒 / 3 回とも同じ
```

**pong が返ること自体が、帯へ出たことの証明になっています。**
画面が「描いた」と返すまでツールは応答を返さない作りにしてあり
（`crates/sshboard-mcp/src/server.rs` の `show`・D16）、返さなければこうなります。

```
{"code":-32603,"message":"sshboard did not confirm the operation on screen
 (0/1 views acknowledged). Refusing to run unseen."}
```

上は、権限（capabilities）を入れる前に実際に出ていた応答です。

### 増えたプロセス

**0 本。**MCP は `sshboard-desktop` のプロセス自身が `127.0.0.1` で listen しています（D15）。
`ps` で見えるのは `sshboard-desktop` 1 つだけです。

### 途中で踏んだこと（Phase 0 で拾えた未知）

1. **Tauri 2 は `capabilities/` を書かないと IPC が通らない。**
   `listen()` は `plugin:event|listen` なので、権限が無いと購読ごと拒否される。
   拒否は画面側で `catch` に落ちるだけで、**Rust 側には何も出ない。**気づきにくい
2. **本番の CSP に `connect-src ... ipc: http://ipc.localhost` が要る。**
   入れ忘れると本番ビルドでだけ IPC が死ぬ。dev では CSP がヘッダに入らないので**気づけない**
3. **描画（`requestAnimationFrame`）を ack の条件にしてはいけない。**
   WKWebView はウィンドウが前面に無いと rAF を止める。この製品は
   「人はエディタを見ていて、AI が裏で動く」使い方が普通なので、
   背面にした瞬間に MCP が全部失敗する。ack は DOM に入った時点で返す（D16）

### まだ埋まっていないもの

- **Windows。**手元は macOS のみ。CI（`.github/workflows/ci.yml`）で Windows の
  ビルドと `cargo test` は回るが、**画面を見ての確認はできない**
- **macOS の目視。**アプリは起動したままにしてあります。帯に `[AI]    ping` の行が
  出ていることを、人が実際に見て判断してください
