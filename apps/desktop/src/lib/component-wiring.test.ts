/**
 * **作った部品は、必ずどこかから描かれていること**（Issue #10 の再発防止）。
 *
 * #10 は「`consoleTerm` を作ったのに、書く線がどこにも無かった」でした。
 * 同じ形の失敗が、部品でも起きます — **作って、繋ぎ忘れる。**
 * 実際に `redaction.svelte.ts` が「書いたまま誰も呼んでいない」状態で
 * しばらく残っていました（`+layout.svelte` から繋いで解消）。
 *
 * 型検査は 1 件も止めません。**使われていない部品は、ただ静かに存在します。**
 *
 * ここが見るのはソースの形だけです。**描かれる条件が正しいかは見ていません。**
 * 止めるのは「1 か所からも参照されていない」という、あの 1 つの壊れ方です。
 */
import { describe, expect, test } from 'vitest';

const components = import.meta.glob('./components/*.svelte', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const sources = import.meta.glob('../**/*.{svelte,ts}', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** `./components/Foo.svelte` → `Foo` */
function nameOf(path: string): string {
	return path.slice(path.lastIndexOf('/') + 1).replace('.svelte', '');
}

describe('部品の配線', () => {
	test('finds the components that exist', () => {
		// **この検査自体が空振りしていないこと。**
		const names = Object.keys(components).map(nameOf);

		expect(names.length).toBeGreaterThanOrEqual(8);
		expect(names).toContain('ConsoleRequestDialog');
	});

	test('renders every component it defines somewhere', () => {
		const orphans = Object.keys(components)
			.map(nameOf)
			.filter((name) => {
				// 自分自身のファイル以外で `<Name` として描かれているか。
				return !Object.entries(sources).some(
					([path, text]) => !path.endsWith(`${name}.svelte`) && text.includes(`<${name}`)
				);
			});

		expect(orphans, `作ったのに、どこからも描かれていない部品: ${orphans.join(', ')}`).toEqual([]);
	});
});

/**
 * **宛先が決まったら、サーバー側の一覧を読むこと**（2026-09-18 に実機で踏みました）。
 *
 * 配布ページ用の写真を撮っていて出ました。接続を開いて［ファイル］を開くと、
 * **サーバー側が「空です」**のまま。実際には 6 件在りました。
 *
 * ```text
 * MCP  : upload, app, .ssh, .bashrc, .bash_profile, .bash_logout
 * 画面 : 空です。
 * ```
 *
 * `onMount` は `loadLocal()`（手元）を呼びますが、**`refresh()`（サーバー側）を
 * 呼んでいませんでした。**しかもタブが 1 本だけだと押しても何も起きません
 * （`switchTo` は `id === session.activeId` で素通りする）ので、
 * **人は F5 か［↻］を押すまで、空だと信じることになります。**
 *
 * **「読んでいない」と「空」を同じ顔で出すのが、この製品で一番悪い形**です。
 */
describe('ファイルの面', () => {
	const browser = components['./components/FileBrowser.svelte'];

	test('finds the file browser', () => {
		expect(browser).toBeTruthy();
	});

	test('reads the server side when the target becomes known', () => {
		// **`session.activeId` を見て読み直す線が在ること。**
		// 在れば、人がタブを押したときも、**AI が宛先を動かしたときも**追従します。
		const watches = /\$effect\(\(\) => \{[\s\S]*?session\.activeId[\s\S]*?\}\)/.test(browser);

		expect(watches).toBe(true);
	});
});
