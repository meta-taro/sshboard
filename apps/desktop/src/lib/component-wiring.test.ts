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

/**
 * **問いの題字が、本文へ重ならないこと**（2026-09-18 に実機で踏みました）。
 *
 * 端末の同意ダイアログを英語で撮ったら、こうなっていました ——
 *
 * ```text
 * The AI wants to use the
 * console          ← **本文の 1 行目に重なっている**
 * If you allow it, the AI can type into this terminal.
 * ```
 *
 * **日本語では出ません。**「AI が端末を使いたいと言っています」は 1 行に収まります。
 * **英語は語が長い**ので、そこだけ折れます。
 *
 * `header { align-items: center }` は**題字が 1 行である前提**の置き方です。
 * 2 行になったとき、**アイコンを中央に合わせようとして高さが伸びません。**
 * **`flex-start` なら、アイコンは 1 行目に付き、題字は素直に下へ伸びます。**
 *
 * **ここは「AI に端末を渡す唯一の同意画面」**です（D42）。
 * **読めない同意は、同意ではありません。**
 */
describe('問いの題字', () => {
	const dialogs = Object.entries(components).filter(([path]) => /Dialog\.svelte$/.test(path));

	test('finds the dialogs', () => {
		expect(dialogs.length).toBeGreaterThanOrEqual(4);
	});

	test('does not lay the heading out with flex', () => {
		// **`align-items` をどちらにしても直りませんでした**（2026-09-21）。
		// **WebKit では flex 容器の高さが中身に追いつかない場面がある** ——
		// 接続タブの名前が切れたのと同じ族です。
		//
		// **アイコンを題字の中へ入れて、普通の行の流れにする。**
		// 行が増えれば高さも増える —— **仕組みで保証されます。**
		const flexed = dialogs
			.filter(([, source]) => {
				const rule = source
					.replace(/\/\*[\s\S]*?\*\//g, '')
					.match(/\n\theader \{[\s\S]*?\n\t\}/)?.[0];
				return rule ? /display:\s*flex/.test(rule) : false;
			})
			.map(([path]) => nameOf(path));

		expect(flexed).toEqual([]);
	});

	test('keeps the icon inside the heading so the box grows with the text', () => {
		// **繋ぎ忘れの見張り。**`header` の規則を消しただけで、
		// markup が `<header><Icon/><h2>` のままだと、**見た目は直りません。**
		const stragglers = dialogs
			.filter(([, source]) => /<header>\s*<Icon/.test(source))
			.map(([path]) => nameOf(path));

		expect(stragglers).toEqual([]);
	});
});
