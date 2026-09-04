/**
 * 「あとで」が版を覚える。
 *
 * いまの `dismiss()` は状態を戻すだけで、版を覚えていません。
 * **α は毎日上げる前提（D30）なので、同じ札が毎日出ます。**
 * 出し続けられた警告は、そのうち中身を読まずに閉じられます。
 */
import { describe, expect, test } from 'vitest';

import { shouldOffer, SKIPPED_STORAGE_KEY } from './skipped';

describe('この版を出すかどうか', () => {
	test('何も覚えていなければ出す', () => {
		expect(shouldOffer('0.1.7', null)).toBe(true);
	});

	test('飛ばした版は、もう出さない', () => {
		expect(shouldOffer('0.1.7', '0.1.7')).toBe(false);
	});

	test('次の版が出たら、また出す', () => {
		// **飛ばしたのは「その版」であって「更新そのもの」ではない。**
		expect(shouldOffer('0.1.8', '0.1.7')).toBe(true);
	});

	test('前後の空白は同じものとして扱う', () => {
		expect(shouldOffer(' 0.1.7 ', '0.1.7')).toBe(false);
		expect(shouldOffer('0.1.7', ' 0.1.7\n')).toBe(false);
	});

	test('壊れた値を覚えていたら、無視して出す', () => {
		// 手で書き換えられていても、**黙って更新を隠さない。**
		expect(shouldOffer('0.1.7', '')).toBe(true);
		expect(shouldOffer('0.1.7', '   ')).toBe(true);
	});

	test('版が名乗られていなければ出す', () => {
		// 判断材料が無いときに黙るのは、いちばんまずい側の間違い。
		expect(shouldOffer('', '')).toBe(true);
		expect(shouldOffer('', '0.1.7')).toBe(true);
	});
});

describe('保存の鍵', () => {
	test('他の設定と同じ流儀で名付ける', () => {
		expect(SKIPPED_STORAGE_KEY).toBe('sshboard-update-skipped');
	});
});
