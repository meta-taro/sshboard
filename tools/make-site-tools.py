#!/usr/bin/env python3
"""配布ページへ出す MCP 道具の一覧を、**コードから**作る。

**手で持つと必ずずれます。**道具を 1 本足してページを直し忘れると、
外へ向けた 1 枚だけが嘘になり、**しかも誰も気づきません。**

出すのは `site/tools.json`。ページの JS がこれを読んで表を描きます。
`pages.yml` が出す前に毎回走らせるので、**古い一覧が配られることはありません。**

道具の説明は**英語のままにします。**これは AI が実際に読む文字列そのもので、
訳して載せると「載っているもの」と「届くもの」が別になります。
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = [
    ROOT / "crates/sshboard-mcp/src/ssh_tools.rs",
    ROOT / "crates/sshboard-mcp/src/server.rs",
]

# **束の名前と並びは人が決める。**コードの並び順は実装の都合で変わるため。
# ここに無い道具が在ったら落とす —— 足したのに分類し忘れた、を通さない。
GROUPS = [
    ("connect", ["connect", "disconnect", "focus_connection", "session_status",
                 "list_connections", "register_connection", "update_connection",
                 "mark_connection"]),
    ("look", ["list_directory", "stat", "read_file", "search", "read_log",
              "disk_usage", "process_list", "network_listen", "service_status",
              "runtime_versions"]),
    ("write", ["make_directory", "upload_file", "write_file"]),
    ("cmds", ["list_readonly_commands", "run_readonly", "list_operations",
              "run_operation"]),
    ("console", ["console_open", "console_type", "console_stop", "read_stream"]),
    ("screen", ["show_view", "capture_window", "pending_status", "await_answer", "diagnostics"]),
    ("itself", ["about_sshboard", "ping"]),
]

TOOL = re.compile(r"#\[tool\((.*?)\)\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", re.S)
DESC = re.compile(r'description\s*=\s*"(.*)"', re.S)
# 1 文だけ出す。**全文は AI 向けで、人には長すぎます。**
SENTENCE = re.compile(r"(?<=[.!?]) ")
LIMIT = 160


def one_line(raw: str) -> str:
    text = re.sub(r"\\\s*\n\s*", "", raw).replace('\\"', '"')
    text = re.sub(r"\s+", " ", text).replace("**", "").strip()
    first = SENTENCE.split(text)[0]
    return first[:LIMIT].rstrip() + "…" if len(first) > LIMIT else first


def collect() -> dict[str, str]:
    found: dict[str, str] = {}
    for path in SOURCES:
        body = path.read_text(encoding="utf-8")
        for attr, name in TOOL.findall(body):
            described = DESC.search(attr)
            if not described:
                sys.exit(f"{name} に description がありません")
            found[name] = one_line(described.group(1))
    return found


def main() -> None:
    found = collect()
    classified = {name for _, names in GROUPS for name in names}

    missing = sorted(set(found) - classified)
    if missing:
        sys.exit(
            f"**分類されていない道具があります**: {missing}\n"
            f"tools/make-site-tools.py の GROUPS へ足してください"
        )
    gone = sorted(classified - set(found))
    if gone:
        sys.exit(f"**GROUPS に在るのにコードに無い道具**: {gone}")

    out = {
        "total": len(found),
        "groups": [
            {"key": key, "tools": [{"name": n, "line": found[n]} for n in names]}
            for key, names in GROUPS
        ],
    }
    target = ROOT / "site/tools.json"
    target.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{len(found)} 本を {target.relative_to(ROOT)} へ書きました")

    # **`llms.txt` の本数も、ここで埋める。**
    #
    # あそこは **AI がこの製品を読む入口**で、手で書くと**道具を増やした日に
    # そこだけ嘘になります。**ページ本体は JS が `tools.json` から読みますが、
    # `llms.txt` は素のテキストなので、生成のときに差し込みます。
    llms = ROOT / "site/llms.txt"
    text = llms.read_text(encoding="utf-8")
    filled = re.sub(r"(\{\{TOOL_COUNT\}\}|\b\d+) tools over MCP", f"{len(found)} tools over MCP", text)
    if filled == text and "tools over MCP" not in text:
        sys.exit("**`llms.txt` に本数を書く場所がありません。**"
                 "`N tools over MCP` の 1 行を残してください")
    llms.write_text(filled, encoding="utf-8")
    print(f"llms.txt の本数を {len(found)} にしました")


if __name__ == "__main__":
    main()
