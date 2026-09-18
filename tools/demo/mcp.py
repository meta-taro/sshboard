#!/usr/bin/env python3
"""sshboard の stdio 中継ごしに MCP を話す、使い捨ての道具。

**合言葉はここで扱いません。**中継が自分で読んで渡します（README の推奨形）。
"""
import json, os, subprocess, sys, threading, queue

# 既定は、入っているもの。**建てたものを撮るときは呼ぶ側が渡します。**
APP = "/Applications/sshboard.app/Contents/MacOS/sshboard"


class Mcp:
    def __init__(self, app=APP, home=None):
        # **合言葉はアプリと同じ HOME の下に在る。**
        # 撮影用の HOME で動かしているときは、中継も同じ所を見ないと合わない。
        env = dict(os.environ)
        if home:
            env["HOME"] = home
        self.p = subprocess.Popen(
            [app, "--mcp-stdio-proxy"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, bufsize=1, env=env,
        )
        self.q = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()
        self.err = []
        threading.Thread(target=self._read_err, daemon=True).start()
        self.n = 0

    def _read(self):
        for line in self.p.stdout:
            line = line.strip()
            if line:
                try:
                    self.q.put(json.loads(line))
                except json.JSONDecodeError:
                    self.err.append("JSON でない行: " + line[:200])

    def _read_err(self):
        for line in self.p.stderr:
            self.err.append(line.rstrip())

    def send(self, method, params=None, want_reply=True):
        msg = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        if want_reply:
            self.n += 1
            msg["id"] = self.n
        self.p.stdin.write(json.dumps(msg) + "\n")
        self.p.stdin.flush()
        if not want_reply:
            return None
        try:
            return self.q.get(timeout=30)
        except queue.Empty:
            raise SystemExit("返事がありません。stderr:\n" + "\n".join(self.err[-10:]))

    def start(self):
        got = self.send("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "shot", "version": "1"},
        })
        self.send("notifications/initialized", want_reply=False)
        return got

    def call(self, name, args=None):
        return self.send("tools/call", {"name": name, "arguments": args or {}})


if __name__ == "__main__":
    m = Mcp()
    info = m.start()
    print("名乗り:", json.dumps(info.get("result", {}).get("serverInfo", {}), ensure_ascii=False))
    for name in sys.argv[1:] or ["list_connections"]:
        got = m.call(name)
        text = got.get("result", {}).get("content", [{}])[0].get("text", "")
        print(f"--- {name} ---")
        print(text[:1500])
