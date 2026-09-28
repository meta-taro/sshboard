#!/usr/bin/env python3
"""**初めて見るホストに MCP から繋いだとき、画面に何が出るか**を撮る（Issue #26）。

    python3 shoot-hostkey.py <HOME> <出し先>

## なぜ要るか

2026-09-28、実運用の席から「**0.1.15 でも `[object Object]` が出る**」と来ました。
配ったタグの中身を調べると、`HostKeyDialog` も橋も**入っています。**
**読んだ限りでは出ようがありません。**

**読んで納得した判断は、この製品で何度も外れています**（D54）。撮ります。

## 報告と同じ経路にする

```
人は触らない   **MCP の connect だけ**を呼ぶ
指紋は書かない  → **初めて見るホスト**になる
```

## 関門

`shoot.py` の関門（架空の 3 件ちょうど）は使えません（ここは 1 件・未接続）。
代わりに **向き先が `127.0.0.1` であることだけ**を見ます。
"""
import base64
import json
import pathlib
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import mcp

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
APP = str(ROOT / "target/release/bundle/macos/sshboard.app/Contents/MacOS/sshboard")
SETTLE = 2.5


def write_config(home: str) -> None:
    """**指紋を書かない**接続を 1 本。誰のサーバーでもない（127.0.0.1）。"""
    d = pathlib.Path(home) / "Library/Application Support/dev.sshboard.sshboard"
    d.mkdir(parents=True, exist_ok=True)
    (d / "connections.toml").write_text(
        "# **初めて見るホストを作るための、架空の接続**（Issue #26 の再現）。\n"
        "# **fingerprint を書きません。**書くと問いが出ません。\n"
        "version = 1\n\n"
        '[[connections]]\nid = "first-time"\nname = "First time"\n'
        'host = "127.0.0.1"\nport = 2222\nuser = "probe"\n'
        'tag = "demo"\ncolor = "amber"\nwrite_roots = []\n',
        encoding="utf-8",
    )


def main() -> None:
    home, out = sys.argv[1], pathlib.Path(sys.argv[2])
    write_config(home)

    m = mcp.Mcp(app=APP, home=home)
    m.start()

    conns = json.loads(m.call("list_connections")["result"]["content"][0]["text"])
    ids = sorted(c["id"] for c in conns)
    if ids != ["first-time"]:
        sys.exit(f"**撮りません。**登録が想定と違う: {ids}")
    print(f"関門 OK（架空 1 件・向き先は 127.0.0.1）")

    # **人は触りません。**報告と同じく、MCP からだけ呼びます。
    got = m.call("connect", {"connection_id": "first-time"})
    print("connect の返り:", json.dumps(got, ensure_ascii=False)[:400])

    # **問いが立っているか**を、AI の口からも見る。
    pending = m.call("pending_status")
    if "result" in pending:
        print("pending_status:", pending["result"]["content"][0]["text"][:300])

    time.sleep(SETTLE)
    out.mkdir(parents=True, exist_ok=True)
    shot = m.call("capture_window", {"redact": False, "max_edge": 1600})
    if "error" in shot:
        sys.exit(f"**撮れませんでした**: {shot['error']['message'][:200]}")
    for c in shot["result"]["content"]:
        if c.get("type") == "image":
            raw = base64.b64decode(c["data"])
            p = out / "hostkey.png"
            p.write_bytes(raw)
            print(f"{p}  {len(raw)} バイト")


if __name__ == "__main__":
    main()
