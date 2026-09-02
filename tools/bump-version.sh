#!/usr/bin/env sh
# バージョンを 3 か所まとめて上げる（D30 / D34）。
#
#   sh tools/bump-version.sh 0.1.1
#
# **同じ番号を 3 つのファイルに書いています。**手で直すと、ずれます。
# ずれても全部ビルドは通り、「配ったファイル名と、アプリが名乗る番号が違う」形で壊れます。
# **自動更新を入れた今、そこは「更新が来ない」「更新し続ける」に化けます。**
#
# ずれ自体は `cargo test -p sshboard-desktop` が捕まえます（`src/version.rs`）。
# このスクリプトは、そもそもずらさないための道具です。
set -eu

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  echo "使い方: sh tools/bump-version.sh <番号>   例: sh tools/bump-version.sh 0.1.1" >&2
  exit 1
fi

# **prerelease をここへ書かせない**（D30）。
# α であることはタグ（v0.1.1-alpha.2）と Release の印で表します。
# 番号そのものに `-alpha` を入れると、3 か所を揃えるのが途端に難しくなります。
if ! printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "番号は 3 つの数だけにしてください（例: 0.1.1）。" >&2
  echo "α であることはタグと Release の印で表します（D30）。指定: $VERSION" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CURRENT="$(grep -m1 '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"

if [ "$CURRENT" = "$VERSION" ]; then
  echo "すでに $VERSION です。何もしません。"
  exit 0
fi

echo "$CURRENT → $VERSION"

# 1. ワークスペースの Cargo.toml（**[workspace.package] の 1 行だけ**）
perl -0pi -e "s/^version = \"\Q$CURRENT\E\"/version = \"$VERSION\"/m" Cargo.toml

# 2. tauri.conf.json（**更新の判定に使われる番号**）
perl -0pi -e "s/\"version\": \"\Q$CURRENT\E\"/\"version\": \"$VERSION\"/" apps/desktop/src-tauri/tauri.conf.json

# 3. package.json
perl -0pi -e "s/\"version\": \"\Q$CURRENT\E\"/\"version\": \"$VERSION\"/" apps/desktop/package.json

echo
echo "書き換えた 3 か所:"
grep -n '^version = ' Cargo.toml
grep -n '"version"' apps/desktop/src-tauri/tauri.conf.json | head -1
grep -n '"version"' apps/desktop/package.json | head -1

echo
echo "**Cargo.lock も更新してください**（版が古いまま残ります）:"
echo "  cargo check --workspace"
echo
echo "揃っているかの確認:"
echo "  cargo test -p sshboard-desktop version"
