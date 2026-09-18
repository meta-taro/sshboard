/**
 * **アイコンが、親を選ばずに済むこと**（Issue #23 / #16）。
 *
 * 実機の報告:
 *
 * > ラベルの上に、字形が出ずに豆腐（□）になっているグリフが出ています。
 * > そのグリフが原因でボタンが 2 行になっています。
 * > その結果、ラベルがボタンの枠からはみ出しています
 *
 * 原因は**フォントではありませんでした。**この部品はインライン SVG です。
 * 行が増えていたのは `display: block` のせいで、
 * **親が flex でないと、アイコンが単独のブロックになって文字を次の行へ押し出します。**
 *
 * アイコン入りのボタンを持つ部品は 4 つあり、**2 つは親が flex、2 つは違いました。**
 * 当たった所へ `display: flex` を足すのではなく、**部品の側を直します** ——
 * 足すだけだと、**次にアイコン入りのボタンを書いた人が同じ穴に落ちます。**
 *
 * ## なぜ書いたものを読むのか
 *
 * **描いて確かめられません。**この環境では Svelte の scoped CSS が DOM へ入らず
 * （`document.querySelectorAll('style').length === 0`）、`getComputedStyle` は
 * UA の既定しか返しません。**通っても落ちても、理由が違う**ので使いません。
 *
 * jsdom には**レイアウトそのものが無い**ので、「2 行になったか」は
 * どのみち測れません。**測れないものを測ったことにしない**ため、
 * ここでは「書いてあること」を見張ります。
 */
import { describe, expect, test } from 'vitest';

// **`node:fs` を使いません。**`svelte-check` の tsconfig に node の型が無く、
// **テストだけが型検査を落とします**（実際に 337 files 1 ERRORS で落ちました）。
// 読み方は `component-wiring.test.ts` に合わせます。
const icon = Object.values(
	import.meta.glob('./components/Icon.svelte', {
		query: '?raw',
		import: 'default',
		eager: true
	}) as Record<string, string>
)[0];

/** `<style>` の中だけ。 */
const styles = icon.slice(icon.indexOf('<style>'));

describe('アイコン（Issue #23 / #16）', () => {
	test('does not lay itself out as a block', () => {
		// **これが #23 の芯です。**親が flex でないボタンの中で単独のブロックに
		// なると、ラベルが 2 行目へ落ち、ボタンの枠からはみ出します。
		expect(styles).not.toMatch(/display:\s*block/);
	});

	test('sits on the same line as the text beside it', () => {
		// flex の中では、flex item は block 化されるので並びは変わりません。
		// **flex の外でだけ、文字の隣に来ます。**
		expect(styles).toMatch(/display:\s*inline-block/);
	});

	test('still refuses to be squeezed in a flex row', () => {
		// **外せません。**外すと、狭い行でアイコンが潰れます。
		expect(styles).toMatch(/flex:\s*none/);
	});
});

/**
 * **接続タブの名前を折り返さないこと**（2026-09-18 に実機で踏みました）。
 *
 * 配布ページ用の写真を撮っていて、運用者に「デザイン崩れてないですか」と
 * 言われて気づきました。**名前が 2 行に折り返し、2 行目が切れていました。**
 *
 * ```text
 * Batch      ← 1 行目
 * (demo)     ← 2 行目が切れている
 * ```
 *
 * 名前に**空白が含まれると**起きます。**実際の接続名は 1 語が多いので、
 * いままで出ていませんでした** —— 架空の設定（`Web (demo)` など）が炙り出しました。
 *
 * 同じファイルの `.scope` と `.hint` には `white-space: nowrap` が在ります。
 * **片方だけ抜けていた**という形です。
 */
import { describe as describeTabs, expect as expectTabs, test as testTabs } from 'vitest';

const fileBrowser = Object.entries(
	import.meta.glob('./components/FileBrowser.svelte', {
		query: '?raw',
		import: 'default',
		eager: true
	}) as Record<string, string>
)[0]?.[1];

describeTabs('接続タブ', () => {
	testTabs('finds the file browser source', () => {
		expectTabs(fileBrowser).toBeTruthy();
	});

	testTabs('does not let a connection name wrap onto a second line', () => {
		// `.conn-tab button` の規則に `white-space: nowrap` が在ること。
		const rule = fileBrowser.match(/\.conn-tab button \{[\s\S]*?\}/)?.[0] ?? '';

		expectTabs(rule).toMatch(/white-space:\s*nowrap/);
	});

	testTabs('keeps a gap between the connection name and its tag', () => {
		// **`Batch runnerdemo` と地続きに出ていました**（2026-09-18・運用者の指摘）。
		//
		// markup では改行で分かれていますが、**Svelte は `{#if}` の周りの空白を落とす**ので、
		// 文字としての隙間は残りません。**並べる側で持ちます。**
		const rule = fileBrowser.match(/\.conn-tab \.tag \{[\s\S]*?\}/)?.[0] ?? '';

		expectTabs(rule).toMatch(/margin-left:/);
	});

	testTabs('does not put flex on the button itself', () => {
		// **`<button>` に flex を載せない**（2026-09-18・実機で切り分けました）。
		//
		// 載せていたとき、**名前の箱だけが中身より狭く決まり**、`Batch ru…` と
		// 詰められていました。**WebKit は `<button>` の中身を無名ブロックで包む**ため、
		// 中身基準の幅が正しく出ません。**Chrome では同じ CSS で 1 件も詰まりません** ——
		// 「規則は正しいのに、出るものが違う」という形でした。
		//
		// **テストでは見つけられません。**画面を撮って初めて出ます（D53）。
		// ここで止められるのは「また載せてしまう」ことだけです。
		const declarations = fileBrowser
			.replace(/\/\*[\s\S]*?\*\//g, '')
			.match(/\.conn-tab button:not\(\.close\) \{[\s\S]*?\}/)?.[0];

		expectTabs(declarations).toBeUndefined();
	});
});
