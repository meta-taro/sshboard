#!/bin/sh
# 手元のテスト用サーバーを建てる。**あなたのサーバーには一切触りません。**
#
#   sh tools/test-server/up.sh        建てる（鍵も作る）
#   sh tools/test-server/up.sh down   片付ける
#
# 鍵は tools/test-server/.key に作ります（gitignore 済み・**使い捨て**）。
set -e
DIR=$(cd "$(dirname "$0")" && pwd)
NAME=sshboard-test-server
PORT=2222

if [ "$1" = "down" ]; then
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  echo "片付けました。"
  exit 0
fi

# 使い捨ての鍵。**実機の鍵とは無関係です。**
if [ ! -f "$DIR/.key" ]; then
  ssh-keygen -t ed25519 -N '' -C 'sshboard-test-server (disposable)' -f "$DIR/.key" >/dev/null
  echo "使い捨ての鍵を作りました: tools/test-server/.key"
fi

docker build -q -t "$NAME" "$DIR" >/dev/null
docker rm -f "$NAME" >/dev/null 2>&1 || true
docker run -d --name "$NAME" -p "$PORT:22" \
  -e AUTHORIZED_KEY="$(cat "$DIR/.key.pub")" "$NAME" >/dev/null

echo "建ちました。127.0.0.1:$PORT / 利用者 probe"
echo "鍵を agent へ:  ssh-add $DIR/.key"
