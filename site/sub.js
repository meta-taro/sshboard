/**
 * 表紙以外のページの言語切り替え（英語と日本語）。
 * **表紙で選んだ言語を引き継ぎます**（同じ `sshboard-site-lang` を読む）。
 * 日本語以外を選んでいる人には英語を出します —— 訳の無い言語で空白を出さない。
 */
(function () {
  const STORED = 'sshboard-site-lang';
  const read = () => {
    try {
      return localStorage.getItem(STORED);
    } catch {
      return null;
    }
  };
  const save = (lang) => {
    try {
      localStorage.setItem(STORED, lang);
    } catch {
      // 保存できなくても、このページでは切り替わる。
    }
  };
  const want = (read() || navigator.language || 'en').toLowerCase().startsWith('ja') ? 'ja' : 'en';

  function show(lang) {
    document.documentElement.lang = lang;
    for (const block of document.querySelectorAll('[data-lang]')) {
      block.hidden = block.getAttribute('data-lang') !== lang;
    }
    for (const button of document.querySelectorAll('.langs button')) {
      button.setAttribute('aria-pressed', String(button.value === lang));
    }
  }

  for (const button of document.querySelectorAll('.langs button')) {
    button.addEventListener('click', () => {
      save(button.value);
      show(button.value);
    });
  }
  show(want);
})();
