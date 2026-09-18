#!/bin/sh
# **配布ページ用の架空の設定へ入れ替える／戻す。**
#
#   sh tools/demo/use-demo.sh        入れ替える（いまの設定は退避）
#   sh tools/demo/use-demo.sh back   戻す
#
# **人の本物の接続一覧を壊さないこと**が、この script の唯一の仕事です。
# 退避先が既に在るときは**上書きしません** —— 2 回走らせて本物を失う事故を防ぎます。
set -eu

case "$(uname -s)" in
  Darwin) DIR="$HOME/Library/Application Support/dev.sshboard.sshboard" ;;
  *)      DIR="${XDG_CONFIG_HOME:-$HOME/.config}/sshboard/sshboard/config" ;;
esac

LIVE="$DIR/connections.toml"
SAVED="$DIR/connections.toml.before-demo"
DEMO="$(cd "$(dirname "$0")" && pwd)/connections.demo.toml"

if [ "${1:-}" = "back" ]; then
  if [ ! -f "$SAVED" ]; then
    echo "退避した設定がありません（$SAVED）。何もしません。"
    exit 1
  fi
  mv "$SAVED" "$LIVE"
  echo "戻しました: $LIVE"
  echo "**アプリを立ち上げ直してください。**読み直しは起動時です。"
  exit 0
fi

mkdir -p "$DIR"
if [ -f "$LIVE" ]; then
  if [ -f "$SAVED" ]; then
    echo "**すでに退避が在ります**（$SAVED）。"
    echo "先に 'sh tools/demo/use-demo.sh back' で戻してください。"
    echo "ここで上書きすると、**本物の設定が消えます。**"
    exit 1
  fi
  cp "$LIVE" "$SAVED"
  echo "いまの設定を退避しました: $SAVED"
fi

cp "$DEMO" "$LIVE"
echo "架空の設定を置きました: $LIVE"
echo "**アプリを立ち上げ直してください。**読み直しは起動時です。"
