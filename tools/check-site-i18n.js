/*
 * 配布ページの訳を検める。**人の目だけでは必ず漏れます。**
 *
 * 見るのは 4 つ。
 *  1. HTML に在るキーが、訳に在るか（逆も）
 *  2. 英語に在る HTML タグが、訳でも閉じているか
 *  3. **紛れ込んだ別の文字**（キリル文字など。見た目が同じで気づけない）
 *  4. 訳の中に数字を焼き込んでいないか（動的に入れる所を潰さない）
 */
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const html = fs.readFileSync(path.join(root, 'site/index.html'), 'utf8');
global.window = {};
require(path.join(root, 'site/i18n.js'));

const used = new Set();
for (const m of html.matchAll(/data-t="([^"$]+)"/g)) used.add(m[1]);
for (const key of Object.keys(window.SSHBOARD_GROUPS)) used.add('tools.' + key);

let bad = 0;
const say = (m) => {
  console.log(m);
  bad++;
};

for (const [code, words] of Object.entries(window.SSHBOARD_I18N)) {
  for (const key of used) {
    if (!(key in words)) say(`${code}: キーが無い  ${key}`);
  }
  for (const key of Object.keys(words)) {
    if (!used.has(key)) say(`${code}: 使われていないキー  ${key}`);
  }
  for (const [key, text] of Object.entries(words)) {
    // 2. タグの数
    const open = (text.match(/<(b|i|code|span|a|br)\b/g) || []).length;
    const close = (text.match(/<\/(b|i|code|span|a)>/g) || []).length;
    const br = (text.match(/<br\s*\/?>/g) || []).length;
    if (open - br !== close) say(`${code}/${key}: タグが閉じていない（開 ${open - br} / 閉 ${close}）`);
    // 3. 紛れ込んだ文字。**ラテン文字の語の中にキリル/ギリシャが混ざる**のを拾う
    const sneaky = text.match(/[a-zA-Z][Ѐ-ӿͰ-Ͽ]|[Ѐ-ӿͰ-Ͽ][a-zA-Z]/);
    if (sneaky && code !== 'ru') say(`${code}/${key}: 別の文字体系が混ざっている  ${sneaky[0]}`);
  }
}

// 4. 道具の本数を訳に焼き込んでいないか
const total = JSON.parse(fs.readFileSync(path.join(root, 'site/tools.json'), 'utf8')).total;
for (const [code, words] of Object.entries(window.SSHBOARD_I18N)) {
  const t = words['tools.intro'];
  if (t && new RegExp(`>\\s*${total}\\s*<`).test(t)) {
    say(`${code}/tools.intro: 本数 ${total} を焼き込んでいる（増えたら嘘になる）`);
  }
}

console.log(bad === 0 ? `検めました。訳 ${Object.keys(window.SSHBOARD_I18N).length} 言語 / キー ${used.size} 個 —— 問題なし` : `**${bad} 件**`);
process.exit(bad === 0 ? 0 : 1);
