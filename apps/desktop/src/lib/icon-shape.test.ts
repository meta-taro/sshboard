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
