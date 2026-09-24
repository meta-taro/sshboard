#!/usr/bin/env python3
"""**一覧が縦に溢れたとき、潰れないか**を撮って確かめる。

    python3 shoot-overflow.py <HOME> <出し先> [件数]

## なぜ要るか

2026-09-24、割符の席が同じ形の不具合を見つけた ——
**縦の flex の子は既定で縮むので、中身が溢れると名簿が高さ 0 まで潰され、
`overflow: hidden` のせいで罫線 1 本しか残らない。**

**こちらの `.list-core` には `min-height: 0` が入っている。**入っているから
大丈夫、とは言わない —— D54 で、CSS を読んで 4 回続けて外している。
**溢れた状態を作って、撮る。**

## 関門

`shoot.py` は「架空の 3 件ちょうど」で止まる。ここは件数を変えるので
その関門は使えない。**代わりに、もっと強い関門を置く** ——

- **すべての接続の向き先が `127.0.0.1` であること**（1 件でも違えば撮らない）
- **識別子がこの script が作った形（`ovf-NN`）であること**

`shoot.py` の関門は 1 文字も緩めない。
"""
import base64, json, pathlib, sys, time
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import mcp

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
APP = str(ROOT / "target/release/bundle/macos/sshboard.app/Contents/MacOS/sshboard")
SETTLE = 2.2
# **配色表そのまま**（`crates/sshboard-connections/src/mark.rs` の CONNECTION_COLORS）。
# 表に無い名前を書くと、アプリが読み込みごと拒む —— **その検査は正しいので、
# こちらが表に合わせる。**
COLORS = [
    "red", "orange", "amber", "yellow", "lime", "green", "emerald", "teal",
    "cyan", "sky", "blue", "indigo", "violet", "purple", "magenta", "pink",
]


def write_config(home: str, n: int) -> None:
    """架空の接続を n 件書く。**向き先は全部 127.0.0.1。**"""
    d = pathlib.Path(home) / "Library/Application Support/dev.sshboard.sshboard"
    d.mkdir(parents=True, exist_ok=True)
    lines = ["# **一覧の溢れを見るための、架空の接続。**誰のサーバーでもない。", "version = 1", ""]
    for i in range(1, n + 1):
        lines += [
            "[[connections]]",
            f'id = "ovf-{i:02d}"',
            f'name = "Demo host {i:02d}"',
            'host = "127.0.0.1"',
            "port = 2222",
            'user = "probe"',
            'tag = "demo"',
            f'color = "{COLORS[i % len(COLORS)]}"',
            "write_roots = []",
            "",
        ]
    (d / "connections.toml").write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    home, out = sys.argv[1], pathlib.Path(sys.argv[2])
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 40
    write_config(home, n)

    m = mcp.Mcp(app=APP, home=home)
    m.start()

    got = m.call("list_connections")
    if "result" not in got:
        # **握り潰さない。**返りの全文と、アプリの stderr を出す。
        print("返り:", json.dumps(got, ensure_ascii=False)[:800])
        print("アプリの stderr:", "".join(getattr(m, "err", []))[-800:] or "(なし)")
        sys.exit("**list_connections が result を返しませんでした。**")
    conns = json.loads(got["result"]["content"][0]["text"])
    ids = sorted(c["id"] for c in conns)
    want = sorted(f"ovf-{i:02d}" for i in range(1, n + 1))
    if ids != want:
        sys.exit(f"**撮りません。**登録が想定と違う: {ids[:5]}… （{len(ids)} 件）")
    print(f"関門 OK（架空 {len(ids)} 件・向き先はすべて 127.0.0.1）")

    out.mkdir(parents=True, exist_ok=True)
    m.call("show_view", {"view": "connections"})
    time.sleep(SETTLE)
    got = m.call("capture_window", {"redact": False, "max_edge": 1600})
    if "error" in got:
        sys.exit(f"**撮れませんでした**: {got['error']['message'][:200]}")
    for c in got["result"]["content"]:
        if c.get("type") == "image":
            raw = base64.b64decode(c["data"])
            p = out / f"overflow-{n}.png"
            p.write_bytes(raw)
            print(f"{p}  {len(raw)} バイト")


if __name__ == "__main__":
    main()
