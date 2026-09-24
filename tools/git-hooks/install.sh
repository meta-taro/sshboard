#!/bin/sh
# 手元ゲートを繋ぐ（product-baseline §5）。**clone した人が 1 回だけ走らせます。**
#
# `.git/hooks/` へ複製せず `core.hooksPath` を向けます。複製方式だと、
# hook を直したときに**入れ直すまで古いものが走り続ける**ので。
set -e
cd "$(dirname "$0")/../.."
git config core.hooksPath tools/git-hooks
echo "繋ぎました: core.hooksPath = $(git config --get core.hooksPath)"
echo "確かめ方 — git commit のとき cargo fmt --all -- --check が走ります。"
