// @vitest-environment jsdom
/**
 * **AI が端末を使いたいときの問い**（D42）を、実際に描いて確かめる。
 *
 * **この製品で初めて、部品を描いて確かめるテストです。**
 *
 * ここが空だったせいで、Issue #10 は「実装済み」のまま配られました
 * （端末に 1 バイトも出ない状態が 3 日続き、`svelte-check` は
 * 283 files, 0 errors のままでした）。**型検査は 1 件も止めません。**
 *
 * この問いは**安全の要**です。人が気づかないまま AI が端末を握る、を止める
 * 最後の一枚なので、**押し間違いで許可されないこと**まで見ます。
 */
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import ConsoleRequestDialog from './ConsoleRequestDialog.svelte';

// **言葉を決め打つ。**テスト環境の既定は `en` に落ちるので、
// 訳が効いていること自体もここで確かめます（**11 言語ぶん入れた分**）。
beforeEach(() => {
	i18n.set('ja');
});

// **描いた分を片付ける。**残すと次のテストで 2 枚目が見つかり、
// **「同じ釦が複数ある」で落ちます**（実際に落ちました）。
afterEach(cleanup);

/** 押し間違いの余地を作らないため、毎回まっさらな相手を用意する。 */
function mount() {
	const onAllow = vi.fn();
	const onDeny = vi.fn();
	render(ConsoleRequestDialog, { props: { onAllow, onDeny } });
	return { onAllow, onDeny };
}

describe('AI が端末を使いたいときの問い（D42）', () => {
	test('says what is being asked, in words the person can act on', () => {
		mount();

		// **何を聞かれているのかが分かること。**
		// 「許可しますか」だけでは、許可すると何が起きるのか分かりません。
		expect(screen.getByRole('alertdialog')).toBeTruthy();
		expect(document.body.textContent).toContain('AI が端末を使いたい');
		expect(document.body.textContent).toContain('取り返せます');
	});

	test('never shows where the connection points or what the AI would type', () => {
		// **接続先を成果物へ書かない**（PRD §8 / CLAUDE.md 禁止事項 4）。
		// この問いは画面写真に写ります。**写ってはいけないものを持たせない。**
		mount();

		const shown = document.body.textContent ?? '';
		for (const leak of ['@', '://', '192.168.', '10.0.', '.com', '.local']) {
			expect(shown.includes(leak), `問いに接続先らしきものが出ている（${leak}）: ${shown}`).toBe(
				false
			);
		}
	});

	test('puts the focus on Refuse, not on Allow', () => {
		// **開いた勢いで Enter を打っても、許可になりません。**
		// 問いは人が始めたものではないので、手が先に動いている可能性があります。
		const { onAllow, onDeny } = mount();

		const focused = document.activeElement as HTMLElement | null;
		expect(focused?.textContent?.trim()).toBe('断る');

		focused?.click();
		expect(onDeny).toHaveBeenCalledTimes(1);
		expect(onAllow).not.toHaveBeenCalled();
	});

	test('treats Escape as an answer, not as a way to make the question vanish', () => {
		// **黙って消さない。**消えると AI は待ち続け、人は問いがあったことすら
		// 知りません。Escape は「断る」として扱います。
		const { onAllow, onDeny } = mount();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

		expect(onDeny).toHaveBeenCalledTimes(1);
		expect(onAllow).not.toHaveBeenCalled();
	});

	test('does not let a click on the backdrop dismiss it', () => {
		// `PassphraseDialog` は背景で閉じます（人が自分で始めた操作の続きなので）。
		// **こちらは人が始めていない問い**なので、誤操作で消させません。
		const { onAllow, onDeny } = mount();

		const backdrop = document.querySelector('.backdrop') as HTMLElement;
		backdrop.click();

		expect(onDeny).not.toHaveBeenCalled();
		expect(onAllow).not.toHaveBeenCalled();
	});

	test('allows only when the person actually presses Allow', () => {
		const { onAllow, onDeny } = mount();

		// **役割で引く。**文字で引くと、本文の「許可すると…」にも当たります
		// （実際に当たりました）。押せる物として引く方が、人の操作に近い。
		screen.getByRole('button', { name: '許可する' }).click();

		expect(onAllow).toHaveBeenCalledTimes(1);
		expect(onDeny).not.toHaveBeenCalled();
	});
});
