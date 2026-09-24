#!/usr/bin/env python3
"""**使っている色の名前が、本当に定義されているか。**

    python3 tools/check-css-tokens.py          # 検査する
    python3 tools/check-css-tokens.py --self   # **この検査自体が効くか**を確かめる

## なぜ要るか

`var(--typo)` は**黙って当たりません。**エラーも警告も出ず、型検査も試験も通り、
**画面を見るまで分かりません。**地の色が当たらなければ親の色がそのまま出るので、
「なんとなく地味」にしか見えない。

割符の席が同じ穴で 4 件出しました（2026-09-24）——

> `var(--bg)` は無い（在るのは `--bg-app` / `-subtle` / `-sunken` / `-elevated`）。
> **地も文字色も当たらず、選んでいる側が周りと同じ見た目になっていました。**
> 残っていたのは `:hover` だけなので、**触ったほうが選ばれて見えた。**

**こちらは同じ日に、同じ症状を別の原因で踏んでいます** —— 暗い配色で
選ばれているタブが帯と差 6 / 255 しか無く、見分けが付きませんでした
（`--surface-raised` で解消・`a7d1128`）。**名前の間違いではありませんでしたが、
見え方は同じでした。**

## なぜ vitest ではなく、ここに置くか

`node:fs` を使うと `@types/node` が要り、**型検査が落ちます。**
Vite の `import.meta.glob` は `{svelte,css}` の波括弧を展開せず、
分けて書いても `.css` を 1 件も拾えませんでした（2026-09-24 に 2 回つまずいた）。
**静かに 0 件になる読み方**は、この検査がいちばん避けたいものです。

## 見張らないもの

- **控えの在る `var(--x, 既定)`** —— 無くても当たるので、間違いではない
- **注記の中** —— 説明に書いた名前を「使っている」と読まない
- **組み立てた名前**（`var(--mark-${color})`）—— 静的には解けない
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "apps/desktop/src"
MIN_FILES = 10
MIN_DEFINED = 20


def strip_notes(text: str) -> str:
    """注記を外す。**説明に書いた名前を、使っていると読まないため。**"""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"<!--.*?-->", "", text, flags=re.S)


def scan(files: dict[str, str]) -> tuple[set[str], dict[str, set[str]]]:
    defined: set[str] = set()
    used: dict[str, set[str]] = {}
    for name, raw in files.items():
        text = strip_notes(raw)
        # CSS の定義と、`style="--x: …"` の両方がこの形。
        defined |= set(re.findall(r"(--[A-Za-z0-9_-]+)\s*:", text))
        # Svelte の `style:--x={…}`。**これも定義。**
        defined |= set(re.findall(r"style:(--[A-Za-z0-9_-]+)", text))
        for m in re.finditer(r"var\(\s*(--[A-Za-z0-9_-]+)\s*([,)])", text):
            if m.group(2) == ",":
                continue  # 控えが在る
            used.setdefault(m.group(1), set()).add(name)
    return defined, used


def missing(files: dict[str, str]) -> list[str]:
    defined, used = scan(files)
    return sorted(
        f"{name}  ←  {', '.join(sorted(where))}"
        for name, where in used.items()
        if name not in defined
    )


def read_sources() -> dict[str, str]:
    out = {}
    for path in sorted(SRC.rglob("*")):
        if path.suffix not in (".svelte", ".css") or ".test." in path.name:
            continue
        out[path.relative_to(SRC).as_posix()] = path.read_text(encoding="utf-8")
    return out


def self_check() -> int:
    """**落ちる側を先に確かめる。**見つけられない検査は、置いてあるだけ。"""
    cases = [
        (
            "植えた名前を見つける",
            {"a.css": ":root { --real: #fff; }",
             "b.svelte": "<style>.x { color: var(--real); background: var(--nope); }</style>"},
            ["--nope  ←  b.svelte"],
        ),
        (
            "控えの在る var(--x, 既定) は落とさない",
            {"c.svelte": "<style>.x { background: var(--nowhere, #0d1013); }</style>"},
            [],
        ),
        (
            "注記の中は使用と読まない",
            {"d.css": "/* var(--only-in-a-note) は数えない */ :root { --a: 1px; }"},
            [],
        ),
        (
            "Svelte の style:--x も定義として数える",
            {"e.svelte": '<div style:--row-mark="red"></div><style>.x{color:var(--row-mark)}</style>'},
            [],
        ),
    ]
    bad = 0
    for label, files, want in cases:
        got = missing(files)
        ok = got == want
        print(f"  {'OK  ' if ok else 'NG  '}{label}")
        if not ok:
            print(f"        欲しい {want}\n        出た   {got}")
            bad += 1
    return bad


def main() -> int:
    if "--self" in sys.argv:
        print("**検査自体を確かめます**")
        bad = self_check()
        print("自己確認: 通りました" if not bad else f"自己確認: **{bad} 件落ちました**")
        return 1 if bad else 0

    files = read_sources()
    defined, used = scan(files)

    # **見張る先が空だから通った、を「0 件だから通った」と区別する。**
    if len(files) < MIN_FILES or len(defined) < MIN_DEFINED:
        print(f"NG 見張る先が細すぎます: ファイル {len(files)} / 定義 {len(defined)}", file=sys.stderr)
        print("   読み方が壊れている可能性があります（**0 件で緑になるのがいちばん危ない**）", file=sys.stderr)
        return 1

    bad = missing(files)
    if bad:
        print(f"NG **定義に無い名前が {len(bad)} 件**:", file=sys.stderr)
        for line in bad:
            print(f"   {line}", file=sys.stderr)
        print("\n   `var(--x)` は黙って当たりません。名前を直すか、tokens.css へ足してください。",
              file=sys.stderr)
        return 1

    print(f"OK ファイル {len(files)} / 定義 {len(defined)} / 控え無しの使用 {len(used)} —— 未定義は 0 件")
    return 0


if __name__ == "__main__":
    sys.exit(main())
