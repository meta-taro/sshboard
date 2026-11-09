/**
 * **どの配布物が、どの OS・CPU 向けか**（表紙と過去の版のページで同じものを使う）。
 *
 * 名前の付け方を 2 か所で解くと、片方だけ直る日が来ます。
 * x64 と ARM64 を「先に見つかった方」で取ると取り違えます（git-qa で実際に起きた）。
 * ブラウザでは `window.SSHBOARD_RELEASES`、試験（node）では `require` で読みます。
 */
(function (root) {
  const has = (name, word) => name.toLowerCase().includes(word);

  /** 1 つの版の資産を、置き場ごとに分ける。 */
  function sortAssets(assets) {
    const list = Array.isArray(assets) ? assets : [];
    const find = (fn) => list.find((a) => fn(a.name));
    return {
      // **`.dmg` を先に。**`.app.tar.gz` は自動更新が食べる形で、人が受け取る形ではない。
      mac:
        find((n) => n.endsWith('.dmg') && has(n, 'universal')) ??
        find((n) => n.endsWith('.dmg')) ??
        find((n) => n.endsWith('.app.tar.gz')),
      winX64: find((n) => n.endsWith('-setup.exe') && has(n, 'x64')),
      winArm: find((n) => n.endsWith('-setup.exe') && has(n, 'arm64')),
      msiX64: find((n) => n.endsWith('.msi') && has(n, 'x64')),
      msiArm: find((n) => n.endsWith('.msi') && has(n, 'arm64')),
    };
  }

  /** mac の配布物が、どの Mac で動くか。 */
  function macLabel(name) {
    if (has(name, 'universal')) return 'universal';
    if (has(name, 'x64') || has(name, 'x86_64')) return 'intel';
    return 'apple-silicon';
  }

  /** 人に配る版だけ（下書きと、自動更新の宛先 `updater` は版ではない・D51）。 */
  function published(list) {
    return (Array.isArray(list) ? list : []).filter((r) => !r.draft && r.tag_name !== 'updater');
  }

  const api = { sortAssets, macLabel, published };
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.SSHBOARD_RELEASES = api;
  }
})(typeof window !== 'undefined' ? window : globalThis);
