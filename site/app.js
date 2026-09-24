/**
 * 配布ページの中身。
 *
 * **外部ファイルにしてあります**（2026-09-24）。インラインだと
 * `script-src 'self'` の CSP が効かず、**外から差し込まれる口を閉じられません。**
 * dbboard の `app.js` と同じ形です。
 *
 * **版・落とす先・道具の本数は、どれも手で書きません。**
 * 書くと、変えるたびにページを直す作業が増え、**忘れた分だけ嘘を配ります。**
 */
// **版・落とす先・道具の一覧は、どれも手で書かない。**
// 書くと、変えるたびにページを直す作業が増え、**忘れた分だけ嘘を配る。**
(async () => {
  const put = (id, text) => {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  };

  // --- 言語 ---------------------------------------------------------
  // **アプリと同じ 11 言語**（`apps/desktop/src/lib/i18n/locales.ts`）。
  // 英語は HTML に直接書いてあるので、**訳が無い言語でも文章は出ます。**
  const LOCALES = [
    ['en', 'English'], ['ja', '日本語'], ['ko', '한국어'],
    ['zh-CN', '简体中文'], ['zh-TW', '繁體中文'], ['de', 'Deutsch'],
    ['fr', 'Français'], ['es', 'Español'], ['pt-BR', 'Português (Brasil)'],
    ['ru', 'Русский'], ['it', 'Italiano'],
  ];
  const STORED = 'sshboard-site-lang';
  const table = window.SSHBOARD_I18N || {};
  const english = new Map();
  for (const el of document.querySelectorAll('[data-t]')) {
    english.set(el, el.innerHTML);
  }

  const resolve = (tag) => {
    if (!tag) return null;
    const codes = LOCALES.map(([c]) => c);
    if (codes.includes(tag)) return tag;
    const primary = tag.split('-')[0].toLowerCase();
    return (
      codes.find((c) => c.toLowerCase() === primary) ??
      codes.find((c) => c.toLowerCase().startsWith(primary + '-')) ??
      null
    );
  };

  let toolCount = '';
  const apply = (code) => {
    const words = code === 'en' ? null : table[code];
    for (const el of document.querySelectorAll('[data-t]')) {
      const key = el.getAttribute('data-t');
      const said = words && words[key];
      el.innerHTML = said ?? english.get(el);
    }
    // **訳を当てると中の span も作り直される。**数はここで入れ直す ——
    // 訳へ数を焼き込むと、道具が増えた日に**その言語だけ嘘になる。**
    put('tool-count', toolCount);
    // **写真も言語に合わせる。**
    //
    // 撮ってあるのは日本語と英語の 2 組だけです。
    // **他の 8 言語には英語の画面を出します** —— 日本語の画面より、
    // まだ読める人が多いためで、**「撮っていない」を隠すためではありません。**
    // （`Not yet` に、残りの言語の写真が無いことを書いてあります。）
    const shots = code === 'ja' ? 'ja' : 'en';
    for (const view of ['connections', 'files', 'console', 'band', 'diag']) {
      const img = document.getElementById(`shot-${view}`);
      if (img) img.src = `./shots/${shots}/${view}.png`;
    }
    document.documentElement.lang = code;
    try {
      localStorage.setItem(STORED, code);
    } catch {
      /* 記憶できないだけ。**画面は出す。** */
    }
  };

  const picker = document.getElementById('lang');
  if (picker) {
    for (const [code, native] of LOCALES) {
      const option = document.createElement('option');
      option.value = code;
      option.textContent = native;
      picker.append(option);
    }
    let stored = null;
    try {
      stored = localStorage.getItem(STORED);
    } catch {
      /* 読めないだけ */
    }
    const start = resolve(stored) ?? resolve(navigator.language) ?? 'en';
    picker.value = start;
    apply(start);
    picker.addEventListener('change', () => apply(picker.value));
  }

  // --- 道具の一覧 ---------------------------------------------------
  // `tools/make-site-tools.py` が `#[tool(` から作ったもの。
  // **道具を足して直し忘れる、が起きない形。**
  try {
    const res = await fetch('./tools.json');
    if (!res.ok) throw new Error(String(res.status));
    const data = await res.json();
    toolCount = String(data.total);
    put('tool-count', toolCount);
    const host = document.getElementById('tools');
    if (host) {
      host.innerHTML = data.groups
        .map(
          (g) =>
            `<h3 data-t="tools.${g.key}">${window.SSHBOARD_GROUPS?.[g.key] ?? g.key}</h3>` +
            '<table class="tools"><tbody>' +
            g.tools
              .map((t) => `<tr><td><code>${t.name}</code></td><td>${t.line}</td></tr>`)
              .join('') +
            '</tbody></table>'
        )
        .join('');
      // **束の見出しも訳す。**描いたあとに当て直す
      apply(picker ? picker.value : 'en');
    }
  } catch {
    const host = document.getElementById('tools');
    if (host) {
      host.innerHTML =
        '<p class="note">The tool list could not be loaded. ' +
        '<a href="https://github.com/meta-taro/sshboard">See the source</a>.</p>';
    }
  }

  // --- 版と落とす先 -------------------------------------------------
  try {
    // **`updater` は版ではなく更新の宛先**なので、数える対象から外す（D51）。
    const res = await fetch(
      'https://api.github.com/repos/meta-taro/sshboard/releases?per_page=10'
    );
    if (!res.ok) throw new Error(String(res.status));
    const list = await res.json();
    const rel = (Array.isArray(list) ? list : []).find(
      (x) => !x.draft && x.tag_name !== 'updater'
    );
    if (!rel) throw new Error('no release');

    put('ver', rel.tag_name);
    const d = new Date(rel.published_at);
    const ymd = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
      d.getDate()
    ).padStart(2, '0')}`;
    put('when', `${rel.tag_name} · ${ymd}`);

    const find = (fn) => (rel.assets || []).find(fn);
    const wire = (linkId, sizeId, asset) => {
      if (!asset) return;
      const a = document.getElementById(linkId);
      if (a) a.href = asset.browser_download_url;
      put(sizeId, `${asset.name} · ${(asset.size / 1048576).toFixed(1)} MB`);
    };
    // **`.dmg` を先に。**`.app.tar.gz` は自動更新が食べる形で、
    // **人が受け取る形ではありません** —— 実際に「変な zip」と踏まれました。
    // 古い版には `.dmg` が無いので、無ければ従来のものへ落とします。
    wire(
      'dl-mac',
      'size-mac',
      find((x) => x.name.endsWith('.dmg')) ?? find((x) => x.name.endsWith('.app.tar.gz'))
    );
    wire('dl-win', 'size-win', find((x) => x.name.endsWith('-setup.exe')));
    wire('dl-msi', 'size-msi', find((x) => x.name.endsWith('.msi')));
  } catch {
    // **黙って古い物を配らない。**読めなかったことを出す
    put('ver', 'see Releases');
    put('when', 'Could not read the release list — check the Releases page.');
  }
})();
