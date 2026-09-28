/**
 * **人に問う箱は、背景を押しても閉じない**（実運用の指摘・2026-09-28）。
 *
 * 実機の言葉 ——
 *
 * > パスワードきいてたじゃないですか？で、**ほかをくりっくするときえちゃう**んですが。
 *
 * **消えても、待ちは残ります。**人には「消えた」だけが見え、AI は
 * `waitingForPassphrase` のまま待ち続けます。**Issue #30 と同じ壊れ方**です
 * （あれは「裏にあって気づけない」、これは「出ていたのに消えた」）。
 *
 * ## なぜ 1 つずつではなく、まとめて見張るか
 *
 * **`ConsoleRequestDialog` の試験だけが、この性質を見ていました。**
 * そこにはこう書いてありました ——
 *
 * > `PassphraseDialog` は背景で閉じます（**人が自分で始めた操作の続きなので**）
 *
 * **その但し書きが、そのまま穴になりました。**AI が問いを立てる口が増えた今、
 * 「人が始めた」は前提にできません。**箱が増えるたびに同じ判断をやり直さないよう、
 * ここで一括して見張ります。**
 *
 * ## 見張らないもの
 *
 * **人が自分で開く箱**（`BundleDialog` / `AboutDialog` など）。
 * あれは「問い」ではなく、閉じても待ちが残りません。
 */
import { describe, expect, test } from 'vitest';

/** **人に問う箱。**AI が立てた問いが、ここに出ます。 */
const ASKING = [
	'PassphraseDialog',
	'ConsoleRequestDialog',
	'OperationRequestDialog',
	'HostKeyDialog'
] as const;

const sources = import.meta.glob('./*.svelte', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** 背景を押したときに閉じる書き方。 */
const CLOSES_ON_BACKDROP = /event\.target\s*===\s*event\.currentTarget/;

describe('人に問う箱', () => {
	test('見張る先が空になっていない', () => {
		// **0 件だから通った、を「読めていないから通った」と区別する。**
		expect(Object.keys(sources).length).toBeGreaterThan(5);
		for (const name of ASKING) {
			expect(sources[`./${name}.svelte`], `${name} を読めていません`).toBeTruthy();
		}
	});

	test.each(ASKING)('%s は、背景を押しても閉じない', (name) => {
		expect(CLOSES_ON_BACKDROP.test(sources[`./${name}.svelte`])).toBe(false);
	});

	test.each(ASKING)('%s には、閉じる道が在る（［やめる］か Escape）', (name) => {
		const text = sources[`./${name}.svelte`];
		// **閉じられないほうも困ります。**答えたくない場面が在ります。
		expect(/Escape/.test(text) || /onCancel|onDeny|onRefuse/.test(text)).toBe(true);
	});
});
