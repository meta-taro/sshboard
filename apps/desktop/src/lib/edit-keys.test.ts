/**
 * 入力欄の切り取り・コピー・貼り付け・全選択（実機の指摘・Windows）。
 *
 * > パスを win で Ctrl+V で打ててなかったように思う
 *
 * **パスフレーズが入れられない ＝ 繋げない**ので、これは一番手前の詰まりです。
 *
 * 自前タイトルバー（D17）で `decorations: false` にした結果、
 * **Windows ではメニューバーごと消えます。**D17 の実行記録にはこう書きました。
 *
 * > 編集（元に戻す・切り取り・コピー・貼り付け）は **WebView が標準で持っている**ので、
 * > キーボードからは従来どおり使える
 *
 * **実機で確かめずに書いた一文**です。メニューの割り当てが消えた以上、
 * **OS 任せにしない**のが確実なので、画面側で受けます。
 *
 * `terminal-clipboard.ts` は **xterm 専用**（`attachCustomKeyEventHandler`）で、
 * 普通の入力欄には効きません。ここはその入力欄の側です。
 */
import { describe, expect, test } from 'vitest';

import { editIntent, type EditKeyEvent } from './edit-keys';

function press(part: Partial<EditKeyEvent>): EditKeyEvent {
	return { key: 'a', ctrlKey: false, metaKey: false, shiftKey: false, inField: true, ...part };
}

describe('editIntent', () => {
	test('takes Ctrl+V as paste where there is no Command key', () => {
		// **Windows で効かなかったのがこれ**です。
		expect(editIntent(press({ key: 'v', ctrlKey: true }), 'other')).toBe('paste');
	});

	test('takes Command+V on macOS', () => {
		expect(editIntent(press({ key: 'v', metaKey: true }), 'mac')).toBe('paste');
	});

	test('handles cut, copy and select-all the same way', () => {
		expect(editIntent(press({ key: 'x', ctrlKey: true }), 'other')).toBe('cut');
		expect(editIntent(press({ key: 'c', ctrlKey: true }), 'other')).toBe('copy');
		expect(editIntent(press({ key: 'a', ctrlKey: true }), 'other')).toBe('selectAll');
	});

	test('does not care about the case the key arrives in', () => {
		// Shift を握ったまま打つと大文字で来ます。**取りこぼさない。**
		expect(editIntent(press({ key: 'V', ctrlKey: true }), 'other')).toBe('paste');
	});

	test('leaves everything alone outside an input', () => {
		// **端末の上では手を出しません。**あちらは `terminal-clipboard.ts` の担当で、
		// **素の Ctrl+C は「走っているものを止める」**です。横取りすると止められません。
		expect(editIntent(press({ key: 'v', ctrlKey: true, inField: false }), 'other')).toBeNull();
		expect(editIntent(press({ key: 'c', ctrlKey: true, inField: false }), 'other')).toBeNull();
	});

	test('ignores keys pressed on their own', () => {
		expect(editIntent(press({ key: 'v' }), 'other')).toBeNull();
		expect(editIntent(press({ key: 'a' }), 'other')).toBeNull();
	});

	test('does not take Ctrl+V on macOS, where Command is the modifier', () => {
		// **mac で Ctrl+V を取ると、端末の慣習と食い違います。**
		expect(editIntent(press({ key: 'v', ctrlKey: true }), 'mac')).toBeNull();
	});

	test('leaves the size shortcuts alone', () => {
		// `Ctrl +` / `Ctrl -` / `Ctrl 0` は文字サイズ。**取り合いにしない。**
		for (const key of ['+', '-', '0', '=', '_']) {
			expect(editIntent(press({ key, ctrlKey: true }), 'other'), key).toBeNull();
		}
	});
});
