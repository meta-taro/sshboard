#!/usr/bin/env python3
"""tauri が束ねるものが、**git に在るか**を確かめる（baseline §23）。

**なぜ要るか**（2026-10-06・他製品の報告から）——

> まっさらな clone で `tauri dev` を走らせると、Windows で `bundle.resources` に
> 載せたファイルが無いと言って止まる。**`tauri dev` も resources の存在を確かめる**ので、
> それらが git に入っておらず build の中でしか作られない作りだと踏む。
> **先に build を走らせた手元では出ない。**

**手元では出ない**のが、この壊れ方のいちばん悪い所です。
参照だけ足してアップロードを忘れると、**ビルドもテストも通ったまま、
実行時にだけ壊れます**（baseline §23 がまさにこれを禁じています）。

sshboard はいま `bundle.resources` を使っていないので踏めませんが、
**使い始めた日に踏みます。**その日に気づけるように、先に見張りを置きます。

標準出力には**パスを出します**（接続先ではないので、公開しても困りません）。
"""

import json
import pathlib
import subprocess
import sys


# **出力を UTF-8 に固定する。**
#
# Windows の Python は標準出力の既定が cp1252 で、**日本語を 1 文字出した瞬間に
# `UnicodeEncodeError` で死にます。2026-09-25 に `check-css-tokens.py` が
# それで Windows の CI を落としており、**この script も同じ所で止まりました**
# （2026-10-06・pre-commit が捕まえました）。
#
# 手元で同じ条件を作れます —— `PYTHONIOENCODING=cp1252 python3 tools/…`
def _force_utf8() -> None:
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8")


_force_utf8()

CONFIG = pathlib.Path("apps/desktop/src-tauri/tauri.conf.json")
# `bundle.icon` / `bundle.resources` は、この場所からの相対で書かれます。
BASE = CONFIG.parent


def tracked() -> set[str]:
    """git が追跡しているパス（リポジトリ直下からの相対）。"""
    said = subprocess.run(
        ["git", "ls-files"], capture_output=True, text=True, check=True
    )
    return set(said.stdout.split())


def referenced(config: dict) -> list[str]:
    bundle = config.get("bundle", {})
    out: list[str] = list(bundle.get("icon") or [])
    resources = bundle.get("resources")
    # `resources` は配列でも地図でも書けます（地図は「元 → 置く先」）。
    if isinstance(resources, dict):
        out += list(resources.keys())
    elif isinstance(resources, list):
        out += resources
    return out


def main() -> int:
    if not CONFIG.exists():
        print(f"{CONFIG} がありません", file=sys.stderr)
        return 1
    config = json.loads(CONFIG.read_text(encoding="utf-8"))

    held = tracked()
    missing: list[str] = []
    for ref in referenced(config):
        # **glob はここでは解きません。**解くと「1 つでも当たれば良し」になり、
        # **作られるはずの物が 0 個でも通ります。**
        if any(ch in ref for ch in "*?["):
            print(f"見張れません（glob）: {ref}")
            continue
        path = (BASE / ref).resolve()
        rel = path.relative_to(pathlib.Path.cwd().resolve()).as_posix()
        if rel not in held:
            missing.append(rel)

    if missing:
        print("**git に入っていないものを束ねようとしています**", file=sys.stderr)
        for rel in missing:
            print(f"  {rel}", file=sys.stderr)
        print(
            "\n`tauri dev` も存在を確かめます。**build の中でしか作られないなら、"
            "まっさらな clone で起動できません**（baseline §23）。\n"
            "git へ入れるか、`beforeDevCommand` で作ってください。",
            file=sys.stderr,
        )
        return 1

    print(f"束ねるもの {len(referenced(config))} 件、すべて git に在ります")
    return 0


if __name__ == "__main__":
    sys.exit(main())
