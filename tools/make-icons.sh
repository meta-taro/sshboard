#!/usr/bin/env sh
# アイコンを作り直す。
#
#   sh tools/make-icons.sh
#
# **元は `apps/desktop/src-tauri/icons/make-icon.py` の 1 本だけ**です
# （家族のルール §8「スクリプトで描く。画像編集ソフトの成果物をコミットしない」）。
# PNG / ICO / ICNS を手で直さないこと — すぐ食い違い、
# **どれが正しいのか誰にも分からなくなります。**
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="apps/desktop/src-tauri/icons/make-icon.py"

[ -f "$ROOT/$SCRIPT" ] || { echo "元のスクリプトがありません: $SCRIPT" >&2; exit 1; }

# 1024px の PNG を 1 枚。**依存を足していません**（math / struct / zlib だけ）。
python3 "$ROOT/$SCRIPT"

cd "$ROOT/apps/desktop"
pnpm exec tauri icon "src-tauri/icons/source.png"

# **Android / iOS は作らない。**対象は Windows と macOS だけ（PRD §7）。
# `tauri icon` は出力を選べないので、出来たものを片付けます。
python3 -c "import shutil,sys; [shutil.rmtree(p, ignore_errors=True) for p in sys.argv[1:]]" \
  src-tauri/icons/android src-tauri/icons/ios

echo
echo "出来ました:"
ls "$ROOT/apps/desktop/src-tauri/icons"
