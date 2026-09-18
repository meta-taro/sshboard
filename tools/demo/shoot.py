#!/usr/bin/env python3
"""配布ページ用の 5 枚を、指定の言語で撮る。

    python3 shoot2.py <HOME> <出し先> [言語タグ]

**撮る前に関門を 2 つ通します。**登録も、開いているものも、架空の 3 件だけ。
ここを飛ばすと、実物の接続が写った写真を公開する事故になります。
"""
import base64, json, pathlib, sys, time
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import mcp

# **建てたものを撮ります。**入っているものではありません ——
# 直した所が入っているかを見たいので。
ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
APP = str(ROOT / "target/release/bundle/macos/sshboard.app/Contents/MacOS/sshboard")
EXPECTED = ["batch-01", "db-01", "web-01"]
VIEWS = ["connections", "files", "console", "band", "diag"]
SETTLE = 2.2


def main():
    home, out = sys.argv[1], pathlib.Path(sys.argv[2])
    m = mcp.Mcp(app=APP, home=home)
    m.start()
    for cid in EXPECTED:
        m.call("connect", {"connection_id": cid})

    ids = sorted(c["id"] for c in json.loads(m.call("list_connections")["result"]["content"][0]["text"]))
    st = json.loads(m.call("session_status")["result"]["content"][0]["text"])
    opened = sorted(o["id"] for o in st["open"])
    if ids != EXPECTED or opened != EXPECTED:
        sys.exit(f"**撮りません。**登録 {ids} ／ 開いている {opened}")
    print(f"関門 OK（宛先 {st['operationsGoTo']}）")

    m.call("focus_connection", {"connection_id": "web-01"})
    m.call("list_directory", {"path": "."})
    m.call("stat", {"path": "."})
    m.call("disk_usage", {})

    out.mkdir(parents=True, exist_ok=True)
    for view in VIEWS:
        m.call("show_view", {"view": view})
        time.sleep(SETTLE)
        got = m.call("capture_window", {"redact": False, "max_edge": 1600})
        if "error" in got:
            print(f"{view}: **エラー** {got['error']['message'][:120]}")
            continue
        for c in got["result"]["content"]:
            if c.get("type") == "image":
                raw = base64.b64decode(c["data"])
                (out / f"{view}.png").write_bytes(raw)
                print(f"{view}.png  {len(raw)} バイト")


if __name__ == "__main__":
    main()
