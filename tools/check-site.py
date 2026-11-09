#!/usr/bin/env python3
"""配布ページ（`site/`）の検め。**書き忘れと、書いてはいけないものを、出す前に落とす。**

紹介ページの構成（oss-notes `shared/lp-structure.md`・2026-11-09）で足した
過去の版・事例・ロードマップ・よくある質問が、**どれか 1 つだけ古くなる**のを防ぎます。

見るもの:
1. 必要なページが在り、どれも題・説明・正規の URL・CSP を持つ
2. sitemap.xml と llms.txt が、在るページを全部（だけ）指している
3. 構造化データ（JSON-LD）が読める。表紙は SoftwareApplication と FAQPage を持ち、
   FAQPage の問いは**画面にも出ている**（画面に無い問いを検索にだけ見せない）
4. 事例のページは、頭に「使うもの・見つかるもの・直し方」の要約を置く
5. 本文に版の数字を書かない（書いた翌日に古くなる・版はリリースから読む）
6. 個人の手がかり（手元のパス・個人のメール）を書かない
"""

import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

for _stream in (sys.stdout, sys.stderr):
    _reconfigure = getattr(_stream, "reconfigure", None)
    if _reconfigure is not None:
        _reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
BASE = "https://meta-taro.github.io/sshboard/"

REQUIRED = [
    "index.html",
    "archive.html",
    "roadmap.html",
    "cases/certificates.html",
    "cases/mail.html",
    "cases/fence.html",
]
CASES = [page for page in REQUIRED if page.startswith("cases/")]

VERSION = re.compile(r"\b0\.\d+\.\d+\b|\bv\d+\.\d+\.\d+\b")
PERSONAL = re.compile(r"/Users/[A-Za-z]|/home/(?!someone\b)[a-z]+/|@gmail\.com|C:\\\\Users\\\\")


class Page(HTMLParser):
    """題・説明・正規の URL・CSP・JSON-LD・見える字を集める。"""

    def __init__(self):
        super().__init__()
        self.meta = {}
        self.canonical = None
        self.title = ""
        self.ld = []
        self.text = []
        self.classes = set()
        self._in = []

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        self._in.append((tag, a.get("type")))
        if tag == "meta":
            key = a.get("name") or a.get("property") or a.get("http-equiv")
            if key:
                self.meta[key.lower()] = a.get("content", "")
        if tag == "link" and a.get("rel") == "canonical":
            self.canonical = a.get("href")
        for cls in (a.get("class") or "").split():
            self.classes.add(cls)

    def handle_endtag(self, tag):
        while self._in:
            if self._in.pop()[0] == tag:
                break

    def handle_data(self, data):
        if not self._in:
            self.text.append(data)
            return
        tag, kind = self._in[-1]
        if tag == "title":
            self.title += data
        elif tag == "script" and kind == "application/ld+json":
            self.ld.append(data)
        elif tag not in ("script", "style"):
            self.text.append(data)


def url_of(page):
    return BASE if page == "index.html" else BASE + page


def main():
    problems = []
    parsed = {}

    for page in REQUIRED:
        path = SITE / page
        if not path.is_file():
            problems.append(f"{page} がありません")
            continue
        p = Page()
        p.feed(path.read_text(encoding="utf-8"))
        parsed[page] = p
        if not p.title.strip():
            problems.append(f"{page}: <title> が空です")
        if not p.meta.get("description"):
            problems.append(f"{page}: 説明（meta description）がありません")
        if p.canonical != url_of(page):
            problems.append(f"{page}: 正規の URL が {url_of(page)} ではありません（{p.canonical}）")
        if "content-security-policy" not in p.meta:
            problems.append(f"{page}: CSP がありません（鍵を扱う道具の入口）")
        for block in p.ld:
            try:
                json.loads(block)
            except json.JSONDecodeError as error:
                problems.append(f"{page}: 構造化データが読めません: {error}")
        text = " ".join(p.text)
        if VERSION.search(text):
            problems.append(f"{page}: 本文に版の数字があります（{VERSION.search(text).group(0)}）。版はリリースから読みます")
        if PERSONAL.search(text):
            problems.append(f"{page}: 個人の手がかりがあります（{PERSONAL.search(text).group(0)}）")

    # 表紙の構造化データ
    index = parsed.get("index.html")
    if index:
        kinds = set()
        faq = []
        for block in index.ld:
            try:
                data = json.loads(block)
            except json.JSONDecodeError:
                continue
            for item in data if isinstance(data, list) else [data]:
                kinds.add(item.get("@type"))
                if item.get("@type") == "FAQPage":
                    faq = [q.get("name", "") for q in item.get("mainEntity", [])]
        for wanted in ("SoftwareApplication", "FAQPage"):
            if wanted not in kinds:
                problems.append(f"index.html: 構造化データに {wanted} がありません")
        visible = " ".join(index.text)
        for question in faq:
            if question not in visible:
                problems.append(f"index.html: よくある質問「{question}」が画面に出ていません")

    # 事例の要約
    for page in CASES:
        p = parsed.get(page)
        if p and "summary" not in p.classes:
            problems.append(f"{page}: 頭に要約の箱（class=\"summary\"）がありません")

    # sitemap と llms.txt
    sitemap = (SITE / "sitemap.xml").read_text(encoding="utf-8")
    listed = set(re.findall(r"<loc>([^<]+)</loc>", sitemap))
    wanted_urls = {url_of(page) for page in REQUIRED}
    for missing in sorted(wanted_urls - listed):
        problems.append(f"sitemap.xml に {missing} がありません")
    for extra in sorted(listed - wanted_urls):
        problems.append(f"sitemap.xml の {extra} は在りません")
    llms = (SITE / "llms.txt").read_text(encoding="utf-8")
    for page in REQUIRED[1:]:
        if url_of(page) not in llms:
            problems.append(f"llms.txt に {url_of(page)} がありません")

    for problem in problems:
        print(f"::error::{problem}")
    if problems:
        sys.exit(1)
    print(f"配布ページ {len(REQUIRED)} 枚を検めました（問題なし）")


if __name__ == "__main__":
    main()
