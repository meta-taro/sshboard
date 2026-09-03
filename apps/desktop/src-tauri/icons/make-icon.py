#!/usr/bin/env python3
"""sshboard のアイコンを描く。

**画像編集ソフトの成果物をコミットしません**（家族のルール §8）。
スクリプトなら、色や形を変えたときに**差分が読めます。**
理由もこのとおり隣に書けます。

    python3 apps/desktop/src-tauri/icons/make-icon.py

1024px の PNG を 1 枚吐きます。全サイズは Tauri が作ります。
**依存は足しません**（`math` / `struct` / `zlib` だけで PNG は書けます）。

## 色

地は **ライトグレー `#E3E7EB`**。決めたのは人（2026-09-03）。
理由は「**ターミナル作業をイメージさせるため**」。筐体の灰色です。

家族で埋まっている色（藍 2 つ・緑・カフェオレ）から離れており、
**Dock に並んだとき無彩色だけが際立ちます。**

地が淡いので、**印は白ではなく濃色**にします（家族のルール §3 の例外条件）。
`#22272E` との明暗差は下の `contrast()` が実測して出します。

## 印

**端末の入力記号（`>` と `_`）。**

この道具の芯は「1 本の SSH を、人と AI が同じ画面で見る」。
その画面とは端末とファイルの面であり、**人が最初に思い浮かべる形が入力記号**です。
dbboard がデータベースのシリンダーを置いたのと同じ考え方で、
**その道具が扱う物そのもの**を描いています。

鍵や盾では、SSH クライアント以外の何にでも付きます（§5「汎用の記号に逃げない」）。
"""

import math
import struct
import zlib
from pathlib import Path

SIZE = 1024

# 家族のルール §3 の数値（出典は git-qa の make-icon.py）
MARGIN = round(SIZE * 0.09)  # 外周の透明な余白 9%
RADIUS = round(SIZE * 0.22)  # 角丸 22%
STROKE = round(SIZE * 0.045)  # 印の線幅 4.5%（これより細いと 32px で消える）

GROUND = (0xE3, 0xE7, 0xEB)
MARK = (0x22, 0x27, 0x2E)

# 印の骨格。**中心 (512, 512) を囲みの中心に置く。**
# `>` は縦長、`_` は横長なので、2 つ合わせた囲みで中心を取ります。
CHEVRON = [(392, 404), (536, 512), (392, 620)]
UNDERSCORE = [(596, 620), (712, 620)]


def relative_luminance(rgb):
    """WCAG の相対輝度。**明暗差を目分量で決めないため。**"""

    def channel(value):
        srgb = value / 255
        return srgb / 12.92 if srgb <= 0.04045 else ((srgb + 0.055) / 1.055) ** 2.4

    r, g, b = (channel(v) for v in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    """明暗差の比。**文字の基準（4.5:1）とは別の話**で、
    ここで見たいのは **32px の Dock で形が読めるか**です（家族のルール §5）。"""
    la, lb = relative_luminance(a), relative_luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def rounded_rect_distance(x, y, left, top, right, bottom, radius):
    """角丸の四角までの符号付き距離。負なら内側。"""
    cx = max(left + radius, min(x, right - radius))
    cy = max(top + radius, min(y, bottom - radius))
    inside_x = left + radius <= x <= right - radius
    inside_y = top + radius <= y <= bottom - radius
    if inside_x or inside_y:
        # 角の外ではないので、辺までの距離で決まる
        return max(left - x, x - right, top - y, y - bottom)
    return math.hypot(x - cx, y - cy) - radius


def segment_distance(x, y, ax, ay, bx, by):
    """線分までの距離。**端は丸めます**（`stroke-linecap: round` と同じ）。"""
    dx, dy = bx - ax, by - ay
    length2 = dx * dx + dy * dy
    t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / length2))
    return math.hypot(x - (ax + t * dx), y - (ay + t * dy))


def coverage(distance):
    """距離を塗り具合へ。**1 画素ぶんでぼかす**（階段を出さないため）。"""
    return max(0.0, min(1.0, 0.5 - distance))


def draw():
    left, top = MARGIN, MARGIN
    right, bottom = SIZE - MARGIN, SIZE - MARGIN

    segments = [
        (*CHEVRON[0], *CHEVRON[1]),
        (*CHEVRON[1], *CHEVRON[2]),
        (*UNDERSCORE[0], *UNDERSCORE[1]),
    ]
    half = STROKE / 2
    # **印の周りだけ距離を測る。**全画素で測ると遅いだけです。
    mark_left = min(min(s[0], s[2]) for s in segments) - STROKE
    mark_right = max(max(s[0], s[2]) for s in segments) + STROKE
    mark_top = min(min(s[1], s[3]) for s in segments) - STROKE
    mark_bottom = max(max(s[1], s[3]) for s in segments) + STROKE

    rows = bytearray()
    for py in range(SIZE):
        y = py + 0.5
        row = bytearray()
        in_mark_band = mark_top <= y <= mark_bottom
        for px in range(SIZE):
            x = px + 0.5
            ground_a = coverage(rounded_rect_distance(x, y, left, top, right, bottom, RADIUS))
            if ground_a <= 0:
                row += b"\x00\x00\x00\x00"
                continue

            mark_a = 0.0
            if in_mark_band and mark_left <= x <= mark_right:
                nearest = min(segment_distance(x, y, *s) for s in segments)
                mark_a = coverage(nearest - half)

            # 印を地の上に重ねる。**地の外へは印を出さない。**
            r = round(GROUND[0] * (1 - mark_a) + MARK[0] * mark_a)
            g = round(GROUND[1] * (1 - mark_a) + MARK[1] * mark_a)
            b = round(GROUND[2] * (1 - mark_a) + MARK[2] * mark_a)
            row += bytes((r, g, b, round(255 * ground_a)))
        rows += b"\x00" + row
    return bytes(rows)


def write_png(path, width, height, rgba_rows):
    def chunk(kind, payload):
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(rgba_rows, 9))
        + chunk(b"IEND", b"")
    )
    Path(path).write_bytes(blob)


def main():
    here = Path(__file__).resolve().parent
    ratio = contrast(GROUND, MARK)
    print(f"地 #{'%02X%02X%02X' % GROUND} / 印 #{'%02X%02X%02X' % MARK}")
    print(f"明暗差（実測）: {ratio:.1f}:1")
    print(f"外周余白 {MARGIN}px（{MARGIN / SIZE:.1%}） / 角丸 {RADIUS}px（{RADIUS / SIZE:.1%}）"
          f" / 線幅 {STROKE}px（{STROKE / SIZE:.1%}）")
    write_png(here / "source.png", SIZE, SIZE, draw())
    print(f"書きました: {here / 'source.png'}")


if __name__ == "__main__":
    main()
