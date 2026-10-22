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

# **出力を UTF-8 に固定する。**
#
# 2026-09-25、この検査そのものが **Windows の CI を落としました** ——
# Windows の Python は標準出力の既定が cp1252 で、**日本語を 1 文字出した瞬間に
# `UnicodeEncodeError` で死にます。**macOS でしか試していませんでした。
#
# 手元で同じ条件を作れます —— `PYTHONIOENCODING=cp1252 python3 tools/…`
def _force_utf8() -> None:
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8")


_force_utf8()

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "apps/desktop/src"
MIN_FILES = 10
MIN_DEFINED = 20
# **配色の塊だと認める下限。**これ未満の `:root` は色の塊ではない
# （`prefers-reduced-motion` の中は 2 個しかない）。
MIN_PALETTE = 20


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


# **テーマに依らないもの。**書体・角丸・動きは暗い側に持たなくてよい。
THEME_FREE = re.compile(r"^--(font|r|shell-pad|ease|fast|slow)")

TOKENS_CSS = SRC / "lib/styles/tokens.css"

# **案が決めた「役」の色**（UI 案・2026-10-21）。
#
# 16 色の印は値を縛りません —— **全部違っていて、地の上で読めること**を
# `check-css-contrast.py` が見ています。縛ると、色を 1 つ調整するたびに
# **CI が落ち、人は検査を書き換えることを覚えます。**
#
# ここに置くのは**意味を担っている色だけ**です。人と AI を入れ替えたら
# 画面の意味が変わるので、**黙って変わってはいけません。**
DESIGN_DARK = {
    "human": "#8cc2ff",
    "ai": "#f5a65b",
    "ground": "#0b0d10",
    "surface": "#13161b",
}


def theme_blocks(text: str) -> list[tuple[str, dict[str, str]]]:
    """`:root` の塊ごとに `--name: value` を拾う。**畳まずに、並びで返す。**

    `check-css-contrast.py` が「**塊を名前で畳まないこと**」と書いている所です。
    最初は見出しを鍵にした辞書で返していて、**明るい側が 2 個になりました** ——
    `@media (prefers-reduced-motion: reduce)` の中にも見出し `:root` の塊が在り、
    `--fast: 0ms` / `--slow: 0ms` の 2 個で**上書きしていた**からです。

    `read_themes` が同じ形で無事だったのは、**16 進数しか拾わないので
    その塊が空になり、捨てられていた**だけです。偶然です。

    暗い配色は 2 か所に在ります（`@media` の中と `[data-theme='dark']`）。
    畳むと**その 2 つがずれても誰も気づきません** ——
    しかも「自動」と「手動で暗い」は**同時に見られない**ので、
    気づくのがいちばん遅い形になります。

    こちらは `rgba(…)` も含めて**全部**拾います（名前の抜けを数えるため）。
    """
    out: list[tuple[str, dict[str, str]]] = []
    for m in re.finditer(r"(:root[^{]*)\{(.*?)\n\t*\}", text, re.S):
        head, body = m.group(1).strip(), re.sub(r"/\*.*?\*/", "", m.group(2), flags=re.S)
        found = dict(re.findall(r"(--[A-Za-z0-9_-]+)\s*:\s*([^;]+);", body))
        if found:
            out.append((head, {k: v.strip().lower() for k, v in found.items()}))
    return out


def theme_trouble(text: str) -> list[str]:
    """明るい側に在る色が暗い側にも在るか、暗い側 2 か所の値が一致するか。"""
    blocks = theme_blocks(text)
    # **配色の塊だけを見る。**`[data-text-size=…]` や `prefers-reduced-motion` の
    # 中の `:root` は色を持たないので、**件数で切り分けます。**
    palettes = [(head, tokens) for head, tokens in blocks if len(tokens) >= MIN_PALETTE]
    if len(palettes) != 3:
        return [
            f"配色の塊が {len(palettes)} 個です（明るい 1 つ ＋ 暗い 2 つ のはず）。"
            f"読み方が壊れている可能性があります —— 見えた塊: "
            + ", ".join(f"{h}({len(t)})" for h, t in blocks)
        ]

    # **並びで決める。**最初の素の `:root` が明るい側。
    # 見出しで選ぶと、`@media` の中が `:root:not([data-theme='light'])` で
    # **dark の字がどこにも無い**ので取りこぼします。
    light_head, light = palettes[0]
    if light_head != ":root":
        return [f"1 つ目の配色が `{light_head}` です（素の `:root` のはず）"]
    darks = palettes[1:]

    bad: list[str] = []
    for name in sorted(light):
        if THEME_FREE.match(name):
            continue
        for head, colors in darks:
            if name not in colors:
                bad.append(f"{head}: **{name} が定義されていません**（明るい側の値がそのまま残ります）")

    (head_a, a), (head_b, b) = darks
    for name in sorted(set(a) | set(b)):
        if a.get(name) != b.get(name):
            bad.append(
                f"**{name} が暗い側 2 か所で違います** —— "
                f"{head_a} が {a.get(name)} / {head_b} が {b.get(name)}"
            )

    for name, want in DESIGN_DARK.items():
        for head, colors in darks:
            got = colors.get(f"--{name}")
            if got != want:
                bad.append(f"{head}: --{name} が案と違います —— 案 {want} / 実物 {got}")
    return bad


# **AI を名乗る選び手が、人の色を使っていないか**（UI 案・2026-10-21）。
#
# フェーズ A で `--accent` を**人の青**（`#8CC2FF`）にした結果、
# それまで `--accent` で描いていた **AI の印が 2 か所で青くなりました** ——
# 帯の AI の行と、端末の面の AI の名乗り。**案とちょうど逆**です。
#
# 「人と AI を色で見分ける」が案の要点なのに、取り違えていました。
# 型検査も試験も通り、**画面を見るまで分かりません。**
HUMAN_COLORS = ("--accent", "--human")
# **子孫の選び手も見ること。**最初は `^\s*\.…` で書いていて、
# `.band li.ai`（空白を含む）を**取りこぼしました** ——
# それが実際に起きた 1 件です（AI の行が人の青で描かれていた）。
AI_SELECTOR = re.compile(r"^[^{}\n]*\.ai\b[^{}\n]*\{", re.M)


def ai_wearing_human(files: dict[str, str]) -> list[str]:
    """`.ai` を名乗る選び手の中で、人の色を使っている所。"""
    bad: list[str] = []
    for name, text in files.items():
        body = strip_notes(re.sub(r"/\*.*?\*/", "", text, flags=re.S))
        for hit in AI_SELECTOR.finditer(body):
            # その規則の中だけを見る（次の `}` まで）。
            end = body.find("}", hit.end())
            rule = body[hit.end() : end if end > 0 else len(body)]
            selector = hit.group(0).strip().rstrip("{").strip()
            for human in HUMAN_COLORS:
                if f"var({human})" in rule:
                    bad.append(f"{name}: `{selector}` が {human} を使っています（AI は --ai）")
    return bad


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

    wearing = ai_wearing_human(files)
    if wearing:
        print(f"NG **AI の印が人の色を使っています（{len(wearing)} 件）**:", file=sys.stderr)
        for line in wearing:
            print(f"   {line}", file=sys.stderr)
        print("\n   **人は `--human` / `--accent`、AI は `--ai`。**案の要点はここです。",
              file=sys.stderr)
        return 1

    trouble = theme_trouble(TOKENS_CSS.read_text(encoding="utf-8"))
    if trouble:
        print(f"NG **配色の不整合が {len(trouble)} 件**:", file=sys.stderr)
        for line in trouble:
            print(f"   {line}", file=sys.stderr)
        print("\n   暗い側は 2 か所に在ります（`@media` と `[data-theme]`）。**両方直してください。**",
              file=sys.stderr)
        return 1

    print(f"OK ファイル {len(files)} / 定義 {len(defined)} / 控え無しの使用 {len(used)} —— 未定義は 0 件")
    print("OK 配色の不整合なし（暗い側 2 か所が一致・案の色・AI は AI の色）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
