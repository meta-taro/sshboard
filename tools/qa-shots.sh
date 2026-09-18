#!/bin/sh
# **画面そのものを、撮って確かめる。**
#
#   sh tools/qa-shots.sh
#
# 運用者の言葉（2026-09-18）——
#
# > つまりこれも重要な検証手法ってことです。MCP でそれだけ重要ってことです。
# > キャプチャ撮る機能は
#
# **1 日でこれだけ出ました。**どれも、撮ろうとしなければ出ていません。
#
#   41c5191  MCP の宛先は web-01、画面は Batch のまま（PRD §4-1 の根拠に関わる）
#   10cde1e  サーバー側を一度も読まず「空です」と嘘をついていた
#   8a6febd  disconnect の説明が引数名を間違えていて、AI は 1 回目に必ず失敗する
#   0cd9cd1  英語の画面が 4 箇所で折れる（日本語では出ない）
#
# ## なぜ git の差分で見るか
#
# **写真はリポジトリに置いてあります。**撮り直して `git diff` に出れば、
# **画面が変わった**ということです。意図した変更なら通し、
# **身に覚えが無ければ、それが崩れ**です。
#
# 画素を比べる道具は入れません。**比べるのは人**です ——
# 「崩れているか」は、機械にはまだ判断できません（baseline §19 / §29）。
#
# ## 撮るのは架空の設定の上だけ
#
# **実物の接続では撮りません**（CLAUDE.md 禁止事項 4 / D26）。
# 中の script が、登録も開いているものも架空の 3 件だけであることを確かめ、
# **合わなければ撮らずに止まります。**
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/target/release/bundle/macos/sshboard.app/Contents/MacOS/sshboard"
SHOOT="$ROOT/tools/demo/shoot.py"

[ -x "$APP" ] || {
  echo "**建てていません。**先に:"
  echo "  pnpm --filter desktop tauri build --bundles app"
  echo "（更新用の署名で落ちますが、.app は出来ます）"
  exit 1
}

nc -z 127.0.0.1 2222 2>/dev/null || {
  echo "**テスト用サーバーが建っていません。**先に:"
  echo "  sh tools/test-server/up.sh"
  exit 1
}

# **確実に終わらせる。**
#
# `osascript -e 'quit app "sshboard"'` だけでは足りません ——
# ここでは**束ねた中の実行体を直に起動している**ので、名前で捕まらないことがあります。
# 残ったまま次を起動すると、**新しい方が口（22022）を取れずに落ち、
# 古い方に繋いだまま撮る**ことになります。
# **それに気づかず「直っていない」と何度も直しにかかりました**（2026-09-18）。
quit_app() {
  osascript -e 'quit app "sshboard"' 2>/dev/null || true
  n=0
  until ! pgrep -f 'MacOS/sshboard' >/dev/null 2>&1 || [ "$n" -ge 10 ]; do
    sleep 1
    n=$((n + 1))
  done
  # まだ居るなら、こちらから止める（起動したのはこの script です）
  if pgrep -f 'MacOS/sshboard' >/dev/null 2>&1; then
    pkill -f 'MacOS/sshboard' 2>/dev/null || true
    n=0
    until ! pgrep -f 'MacOS/sshboard' >/dev/null 2>&1 || [ "$n" -ge 10 ]; do
      sleep 1
      n=$((n + 1))
    done
  fi
  pgrep -f 'MacOS/sshboard' >/dev/null 2>&1 && {
    echo "**止められません。**手で終了してから、もう一度走らせてください"
    exit 1
  }
  # **口が空いたことまで確かめる。**空く前に起動すると、次が落ちます
  n=0
  until ! nc -z 127.0.0.1 22022 2>/dev/null || [ "$n" -ge 10 ]; do
    sleep 1
    n=$((n + 1))
  done
  return 0
}

wait_mcp() {
  n=0
  until [ "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:22022/mcp 2>/dev/null)" = "401" ] \
    || [ "$n" -ge 30 ]; do
    sleep 1
    n=$((n + 1))
  done
}

# **言語ごとに別の HOME。**localStorage に前の言語が残ると、次が英語にならない。
shoot_one() {
  lang="$1"
  home="$2"
  langs="$3"

  mkdir -p "$home/Library/Application Support/dev.sshboard.sshboard"
  cfg="$home/Library/Application Support/dev.sshboard.sshboard/connections.toml"
  cp "$ROOT/tools/demo/connections.demo.toml" "$cfg"

  # **ホスト鍵の指紋は、いま建っているサーバーから取る。**
  #
  # リポジトリの架空設定には書けません —— `up.sh` が**環境ごとに鍵を作る**ので、
  # 書くと他の人の手元で必ず食い違います。
  #
  # 書かないと初回に「初めて見るホストです」で止まります。**それは正しい動き**で、
  # **AI には答えられません**（D47）。ここは撮影のための下ごしらえであって、
  # **確認を省いているのではありません** —— 指紋は、いま繋ぐ相手から取っています。
  fp="$(ssh-keyscan -t ed25519 -p 2222 127.0.0.1 2>/dev/null | ssh-keygen -lf - 2>/dev/null | awk '{print $2}')"
  [ -n "$fp" ] || { echo "**テスト用サーバーの指紋を取れません。**"; exit 1; }
  awk -v fp="$fp" '{ print; if ($0 ~ /^user = /) print "fingerprint = \"" fp "\"" }' \
    "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"

  quit_app
  HOME="$home" "$APP" -AppleLanguages "$langs" >"$home/sshboard.log" 2>&1 &
  started=$!
  wait_mcp
  # **撮る相手が、いま起動したものであること。**
  # 古いものが残っていると、**直したはずの画面が直っていないように見えます。**
  kill -0 "$started" 2>/dev/null || {
    echo "**起動したものが落ちています。**口が空いていなかった可能性があります"
    tail -5 "$home/sshboard.log" || true
    exit 1
  }
  python3 "$SHOOT" "$home" "$ROOT/site/shots/$lang"
}

# **毎回まっさらな置き場所を使う。**
#
# 使い回すと、**WebView が index.html ごとキャッシュ**します。
# CSS のファイル名が変わっても、**古い index.html が古い CSS を指したまま**で、
# **直したはずの画面が一度も描かれません。**
#
# 実際にこれで 2 回、**CSS を変えたのに写真がバイト単位で同一**になり、
# 「直っていない」と思い込んで無駄に直しにかかりました（2026-09-18）。
# **同じ絵が返ってきたら、まず「描かれているか」を疑うこと。**
# **短い道にする。**`mktemp` の既定は `/var/folders/...` の長い道で、
# **それが画面に出て、右の面まで押しました**（2026-09-18）。
# 写真に写るものなので、**読める長さ**にします。
STAGE="/tmp/sshboard-qa-$$"
mkdir -p "$STAGE"
echo "撮影用の置き場所: $STAGE"

echo "=== 日本語 ==="
shoot_one ja "$STAGE/ja" '(ja)'
echo "=== 英語 ==="
shoot_one en "$STAGE/en" '(en)'
quit_app

echo
echo "撮り終えました。**変わった所を見てください。**"
echo
git -C "$ROOT" status --short site/shots/ || true
echo
echo "意図した変更なら、そのまま commit してください。"
echo "**身に覚えが無ければ、それが崩れです。**"
