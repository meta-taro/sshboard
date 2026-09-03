#!/usr/bin/env sh
# アイコンを作り直す。
#
#   sh tools/make-icons.sh
#
# **元は `apps/desktop/src-tauri/icons/source.svg` の 1 枚だけ**です。
# PNG / ICO / ICNS を手で直さないこと — すぐ食い違い、
# **どれが正しいのか誰にも分からなくなります。**
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="apps/desktop/src-tauri/icons/source.svg"

[ -f "$ROOT/$SOURCE" ] || { echo "元の SVG がありません: $SOURCE" >&2; exit 1; }

cd "$ROOT/apps/desktop"
pnpm exec tauri icon "src-tauri/icons/source.svg"

# **Android / iOS は作らない。**対象は Windows と macOS だけ（PRD §7）。
# `tauri icon` は出力を選べないので、出来たものを片付けます。
python3 -c "import shutil,sys; [shutil.rmtree(p, ignore_errors=True) for p in sys.argv[1:]]" \
  src-tauri/icons/android src-tauri/icons/ios

echo
echo "出来ました:"
ls "$ROOT/apps/desktop/src-tauri/icons"
