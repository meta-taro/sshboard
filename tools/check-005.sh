#!/bin/sh
# Issue 005 の確認。**動いている sshboard に対して走らせます。**
#
# 使い方:
#   1. sshboard を起動する
#   2. 画面の「Phase 0 の確認用に流す」を押す（**人の操作です**）
#   3. sh tools/check-005.sh http://127.0.0.1:<ポート>/mcp
#
# ポートはアプリの画面か、起動した端末の `MCP listening on ...` に出ています。
#
# **接続先は扱いません。**Phase 0 の確認用の出力は合成で、サーバーへ繋いでいません。
set -e
U="$1"
[ -n "$U" ] || { echo "使い方: sh tools/check-005.sh <MCP の URL>"; exit 2; }

H1='Content-Type: application/json'
H2='Accept: application/json, text/event-stream'

SID=$(curl -sS -D /tmp/sb005_h.txt -H "$H1" -H "$H2" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"check-005","version":"0"}}}' \
  "$U" -o /dev/null; grep -i '^mcp-session-id:' /tmp/sb005_h.txt | tr -d '\r' | awk '{print $2}')

curl -sS -o /dev/null -H "$H1" -H "$H2" -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' "$U"

BODY=$(curl -sS -H "$H1" -H "$H2" -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_stream","arguments":{}}}' \
  "$U" | grep '^data: {' | sed 's/^data: //')

echo "$BODY"
echo

# ANSI が混ざっていないか。JSON の中では 6 文字の並びとして出てくる。
if printf '%s' "$BODY" | grep -q 'u001b'; then
  echo "NG: MCP 側の文字列に ANSI エスケープが混ざっています"
  exit 1
fi
if printf '%s' "$BODY" | grep -q '"text":""'; then
  echo "まだ何も流れていません。画面の「Phase 0 の確認用に流す」を押してから、もう一度走らせてください。"
  exit 0
fi

echo "OK: MCP 側の文字列に ANSI エスケープはありません"
echo
echo "**GUI 側に色が残っているかは、画面を見た人が判断してください**（product-baseline §19）。"
