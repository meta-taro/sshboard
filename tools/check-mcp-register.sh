#!/bin/sh
# MCP の登録が「どのフォルダからでも」見えるかを、本物の Claude Code で確かめる（2026-10-25）。
#
# Windows で「/mcp しても一覧に出てこない」と言われた。claude mcp add の既定は local
# ＝打ったフォルダだけ。README と釦のコマンドに --scope user を付けたので、それが
# 本当に効くかを、登録 → 3 つのフォルダで一覧 → 消す、で測る。
#
# 要るもの: claude（Claude Code の CLI）と、起動している sshboard。
# 合言葉は使わない（--mcp-stdio-proxy で中継するので、~/.claude.json に残らない）。
#
#   sh tools/check-mcp-register.sh [sshboard の実行ファイル]
set -eu

EXE="${1:-/Applications/sshboard.app/Contents/MacOS/sshboard}"
NAME="sshboard-register-check"
[ -x "$EXE" ] || { echo "実行ファイルがありません: $EXE" >&2; exit 2; }
# 登録は別のフォルダから起動されるので、相対の道は通らない。
EXE="$(cd "$(dirname "$EXE")" && pwd)/$(basename "$EXE")"
command -v claude >/dev/null || { echo "claude（Claude Code の CLI）がありません" >&2; exit 2; }

if claude mcp get "$NAME" >/dev/null 2>&1; then
  echo "$NAME が既に登録されています。消してから走らせてください" >&2
  exit 2
fi

WORK="$(mktemp -d)"
cleanup() {
  claude mcp remove --scope user "$NAME" >/dev/null 2>&1 || true
  rm -r "$WORK"
}
trap cleanup EXIT INT TERM

mkdir -p "$WORK/a" "$WORK/b"
(cd "$WORK/a" && claude mcp add --scope user "$NAME" -- "$EXE" --mcp-stdio-proxy >/dev/null)

FAILED=0
for dir in "$WORK/a" "$WORK/b" "$HOME"; do
  line="$(cd "$dir" && claude mcp list 2>&1 | grep "^$NAME:" || true)"
  case "$line" in
    *"✔ Connected"*) echo "OK  $dir" ;;
    "") echo "NG  $dir —— 一覧に出ない（登録がこのフォルダから見えない）"; FAILED=1 ;;
    *) echo "NG  $dir —— ${line#*- }"; FAILED=1 ;;
  esac
done
exit "$FAILED"
