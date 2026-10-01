/**
 * **どの欄が悪いのかを、検証が指す**（オーナーの依頼・2026-10-01）。
 *
 * 実機の言葉 ——
 *
 * > 接続を作ろうとしたところ、「識別子は英数字と . _ - だけにしてください」と
 * > 表示されました。**文言は出ていますが、エラーだと一目で分かりません。**
 *
 * 依頼は「**赤字で出す／問題のある入力欄に赤い枠線か薄い赤の背景**」。
 * **後者には「どの欄か」が要ります。**いまの検証は文言の鍵しか返していませんでした。
 *
 * （文字の色が薄い灰色（`--fg-faint`）だったことが、一目で分からない直接の原因です。
 * それは画面側で直します。ここは**欄を指せるようにする**分。）
 */
import { describe, expect, test } from 'vitest';

import { emptyConnection, whyNotSavable, type Connection } from './connections';

function usable(over: Partial<Connection> = {}): Connection {
	return { ...emptyConnection(), id: 'web', host: 'example.invalid', user: 'ops', port: 22, ...over };
}

describe('止めている欄を指す', () => {
	test('points at the id when it is empty', () => {
		expect(whyNotSavable(usable({ id: '' }), [])?.field).toBe('id');
	});

	test('points at the id when it has characters that are not allowed', () => {
		expect(whyNotSavable(usable({ id: 'web prod' }), [])?.field).toBe('id');
	});

	test('points at the id when it is already taken', () => {
		expect(whyNotSavable(usable({ id: 'web' }), ['web'])?.field).toBe('id');
	});

	test('points at the host, the user, the port and the tag', () => {
		expect(whyNotSavable(usable({ host: '' }), [])?.field).toBe('host');
		expect(whyNotSavable(usable({ user: '' }), [])?.field).toBe('user');
		expect(whyNotSavable(usable({ port: 0 }), [])?.field).toBe('port');
		expect(whyNotSavable(usable({ tag: 'あ'.repeat(99) }), [])?.field).toBe('tag');
	});

	test('says nothing when there is nothing wrong', () => {
		expect(whyNotSavable(usable(), [])).toBeNull();
	});
});
