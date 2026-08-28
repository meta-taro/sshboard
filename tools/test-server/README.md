# 手元のテスト用サーバー

**あなたのサーバーには一切触りません。**Docker で sshd を建てます（product-baseline §3）。

```sh
sh tools/test-server/up.sh          # 建てる（使い捨ての鍵も作る）
ssh-add tools/test-server/.key      # agent へ載せる
sh tools/test-server/up.sh down     # 片付ける
```

`127.0.0.1:2222` / 利用者 `probe`。

## なぜ本番と同じ系統か

実機は **OpenSSH 8.7 / 9.9 の RHEL 系**でした（Issue 002）。
AlmaLinux 9 は RHEL 9 系なので、**同じ踏み方をします。**

## 何を「わざと」再現しているか

**繋がることではありません。**繋がるだけなら何でもよい。

| 再現しているもの | なぜ |
|---|---|
| `/var/log/maillog` が root しか読めない | **実機で踏んだ**（Issue 002）。D20 の `setfacl` をここで試す |
| `/var/log/japanese-euc.log` が **EUC-JP** | ログが UTF-8 とは限らない。`xterm.js` も `read_file` も UTF-8 前提 |
| `/home/probe/app/logs/app.log` が**増え続ける** | `tail -f` を流す（Issue 005）。色付きの行も混ぜてある |
| 利用者が読めるアプリのログがある | **実務ではここが読めれば足りる場合がある** |

## 鍵について

`tools/test-server/.key` は **`up.sh` が作る使い捨ての鍵**で、gitignore 済みです。
**実機の鍵とは無関係**で、このコンテナ以外のどこにも通用しません。
