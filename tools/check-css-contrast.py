#!/usr/bin/env python3
"""**字と地の対比を、機械が数える。**

    python3 tools/check-css-contrast.py          # 検査する
    python3 tools/check-css-contrast.py --self   # **この検査自体が効くか**を確かめる

## なぜ要るか

2026-09-24、暗い配色で**選ばれているタブが帯と差 6 / 255 しか無く、
どれを押したか分かりませんでした**（実機の指摘）。直したとき、こちらは
**明るさの差**で値を選びました —— **対比比は 1 度も数えていません。**
（結果は 11.54 : 1 で足りていましたが、**確かめずに通していました。**）

割符の席は同じ検査を入れた直後、**自分が入れたばかりのタブが 2.76 : 1 で
落ちた**そうです。**目で見て決めた値は、しばしば足りません。**

## 総当たりにしない

色を全部の組で掛け合わせると、**実際には隣り合わない組**で落ちて、
落ちること自体に意味が無くなります。**実際に組にして使っているものだけ**を
ここに並べます。**並べた所が、そのまま「どこを見ているか」の一覧になります。**

新しい組を作ったら、ここへ 1 行足してください。**足し忘れは、この検査では
捕まりません** —— そこは `check-css-tokens.py` と違う所です。
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
TOKENS = ROOT / "apps/desktop/src/lib/styles/tokens.css"

# 本文の目安。WCAG 2.1 AA。
NEED_TEXT = 4.5
# 補助的な字（小さく薄いもの）の目安。
NEED_FAINT = 3.0

# **実際に組にして使っているものだけ。**（字, 地, 目安, どこで使っているか）
PAIRS = [
    ("fg", "surface-raised", NEED_TEXT, "選ばれているタブ（.tabs button.active）"),
    ("fg-muted", "shell", NEED_TEXT, "選ばれていないタブ（.tabs button）"),
    ("fg", "surface", NEED_TEXT, "本文・札の中身"),
    ("fg-muted", "surface", NEED_TEXT, "控えめな字（説明・補足）"),
    ("fg-faint", "surface", NEED_FAINT, "薄い字（識別子・道）"),
    ("fg", "surface-2", NEED_TEXT, "触れている行・入力の地"),
    ("accent-fg", "accent", NEED_TEXT, "強調のボタンの字"),
]


def linear(channel: int) -> float:
    c = channel / 255
    return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4


def luminance(hex_color: str) -> float:
    h = hex_color.lstrip("#")
    r, g, b = (int(h[i : i + 2], 16) for i in (0, 2, 4))
    return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)


def contrast(a: str, b: str) -> float:
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def read_themes(text: str) -> dict[str, dict[str, str]]:
    """`:root` の塊ごとに、**16 進数で書かれた色だけ**を拾う。

    `rgba(…)` や `color-mix(…)` は地の上に重ねる指定で、**単独では色が決まりません。**
    ここでは数えません。

    **塊を名前で畳まないこと。**最初はそうして、`--surface-raised` を 1 つ目の
    暗い塊だけ悪くした探り棒が**素通りしました** —— 2 つ目の塊が上書きして、
    悪い値が消えていたからです（2026-09-24）。

    暗い配色は 2 か所に在ります（`@media` の中と `[data-theme='dark']`）。
    **畳むと、その 2 つがずれても誰も気づきません。**塊ごとに数えます。
    """
    themes: dict[str, dict[str, str]] = {}
    for m in re.finditer(r"(:root[^{]*)\{(.*?)\n\t*\}", text, re.S):
        head, body = m.group(1), m.group(2)
        found = dict(re.findall(r"(--[A-Za-z0-9_-]+)\s*:\s*(#[0-9a-fA-F]{6})\s*;", body))
        if not found:
            continue
        themes[head.strip()] = {k[2:]: v for k, v in found.items()}
    return themes


def check(themes: dict[str, dict[str, str]]) -> list[str]:
    bad = []
    for theme_name, colors in themes.items():
        for fg, bg, need, where in PAIRS:
            if fg not in colors or bg not in colors:
                bad.append(f"{theme_name}: --{fg} か --{bg} が見つかりません（{where}）")
                continue
            r = contrast(colors[fg], colors[bg])
            if r < need:
                bad.append(
                    f"{theme_name}: {where}  --{fg} {colors[fg]} on --{bg} {colors[bg]}"
                    f"  = **{r:.2f} : 1**（目安 {need}）"
                )
    return bad


def self_check() -> int:
    cases = [
        ("足りない組を見つける",
         {":root[data-theme='dark']": {"fg": "#3a3f45", "surface-raised": "#262e3a", "fg-muted": "#a3adb9",
                       "shell": "#12161b", "surface": "#171c22", "fg-faint": "#737d89",
                       "surface-2": "#1c222a", "accent-fg": "#06251c", "accent": "#4fd6ae"}},
         True),
        ("足りている組は通す",
         {":root[data-theme='dark']": {"fg": "#e8ecf1", "surface-raised": "#262e3a", "fg-muted": "#a3adb9",
                       "shell": "#12161b", "surface": "#171c22", "fg-faint": "#737d89",
                       "surface-2": "#1c222a", "accent-fg": "#06251c", "accent": "#4fd6ae"}},
         False),
        ("名前が無ければ、通さずに言う",
         {":root[data-theme='dark']": {"fg": "#e8ecf1"}},
         True),
    ]
    bad = 0
    for label, themes, want_fail in cases:
        got = check(themes)
        ok = bool(got) == want_fail
        print(f"  {'OK  ' if ok else 'NG  '}{label}")
        if not ok:
            print(f"        出た {got}")
            bad += 1
    # 計算そのものも 1 つ確かめる（白と黒は 21 : 1）
    r = contrast("#ffffff", "#000000")
    ok = abs(r - 21.0) < 0.01
    print(f"  {'OK  ' if ok else 'NG  '}白と黒が 21 : 1（出た {r:.2f}）")
    return bad + (0 if ok else 1)


def main() -> int:
    if "--self" in sys.argv:
        print("**検査自体を確かめます**")
        bad = self_check()
        print("自己確認: 通りました" if not bad else f"自己確認: **{bad} 件落ちました**")
        return 1 if bad else 0

    themes = read_themes(TOKENS.read_text(encoding="utf-8"))
    # **見張る先が空だから通った、を区別する。**
    # **塊は 3 つ**（明るい :root ／ @media の暗い側 ／ [data-theme='dark']）。
    # 減っていたら、読めていないか、片方が消えています。
    if len(themes) < 3:
        print(f"NG 配色の塊が {len(themes)} つしか読めていません（明 1・暗 2 の 3 つが要ります）",
              file=sys.stderr)
        for head in themes:
            print(f"   読めた: {head}", file=sys.stderr)
        return 1
    bad = check(themes)
    if bad:
        print(f"NG **対比が足りない組が {len(bad)} 件**:", file=sys.stderr)
        for line in bad:
            print(f"   {line}", file=sys.stderr)
        return 1
    n = sum(len(c) for c in themes.values())
    print(f"OK 配色 {len(themes)} つ / 色 {n} 個 / 見た組 {len(PAIRS) * len(themes)} —— 足りない組は 0 件")
    return 0


if __name__ == "__main__":
    sys.exit(main())
