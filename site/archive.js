/**
 * **過去の版の一覧**（紹介ページの構成 2）。GitHub のリリースから読みます。
 * 版を出すたびにこのページを直さない —— 手で書いた一覧は、書き忘れた分だけ嘘になります。
 */
(async () => {
  const host = document.getElementById('archive');
  if (!host) return;
  const { sortAssets, macLabel, published } = window.SSHBOARD_RELEASES;
  // 内部の札（`universal` など）を、そのまま人に見せない。
  const MAC_WORDS = { universal: 'Universal (Apple Silicon + Intel)', 'apple-silicon': 'Apple Silicon', intel: 'Intel' };

  const link = (asset, label) => {
    if (!asset) return '—';
    const mb = (asset.size / 1048576).toFixed(1);
    const a = document.createElement('a');
    a.href = asset.browser_download_url;
    a.textContent = label ? `${label} · ${mb} MB` : `${mb} MB`;
    return a;
  };
  const cell = (content) => {
    const td = document.createElement('td');
    if (typeof content === 'string') td.textContent = content;
    else td.append(content);
    return td;
  };

  try {
    const res = await fetch('https://api.github.com/repos/meta-taro/sshboard/releases?per_page=100');
    if (!res.ok) throw new Error(String(res.status));
    const list = published(await res.json());
    host.replaceChildren();
    for (const rel of list) {
      const sorted = sortAssets(rel.assets);
      const tr = document.createElement('tr');
      const name = document.createElement('a');
      name.href = rel.html_url;
      name.textContent = `${rel.tag_name} · ${String(rel.published_at || '').slice(0, 10)}`;
      tr.append(
        cell(name),
        cell(link(sorted.mac, sorted.mac ? MAC_WORDS[macLabel(sorted.mac.name)] : '')),
        cell(link(sorted.winX64, '')),
        cell(link(sorted.winArm, ''))
      );
      host.append(tr);
    }
  } catch {
    // **黙って空の表を出さない。**読めなかったと言う。
    host.innerHTML =
      '<tr><td colspan="4" class="note">Could not read the release list. See the <a href="https://github.com/meta-taro/sshboard/releases">Releases page</a>.</td></tr>';
  }
})();
