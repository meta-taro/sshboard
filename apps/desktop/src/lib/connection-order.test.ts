import { describe, expect, test } from 'vitest';

import { dropTarget, gapForPointer, moveTarget, reorder } from './connection-order';

describe('▲▼ で 1 つ動かす', () => {
	test('上へ', () => expect(moveTarget(2, -1, 5)).toBe(1));
	test('下へ', () => expect(moveTarget(2, 1, 5)).toBe(3));

	test('先頭から上へは行けない', () => {
		// **ボタンを無効にする根拠がこれ。**押せて何も起きない、を作らない。
		expect(moveTarget(0, -1, 5)).toBeNull();
	});

	test('末尾から下へは行けない', () => expect(moveTarget(4, 1, 5)).toBeNull());

	test('一覧の外は断る', () => {
		expect(moveTarget(-1, 1, 5)).toBeNull();
		expect(moveTarget(5, -1, 5)).toBeNull();
	});
});

describe('掴んで落とす — 隙間を位置へ直す', () => {
	test('上の方へ落とすと、隙間がそのまま位置になる', () => {
		expect(dropTarget(3, 1, 5)).toBe(1);
	});

	test('下の方へ落とすと、1 つ詰まる', () => {
		// **掴んだ行がまだ一覧に居る**ので、自分より下の隙間は 1 つ大きい。
		expect(dropTarget(1, 4, 5)).toBe(3);
	});

	test('いちばん上（隙間 0）へ落とせる', () => expect(dropTarget(3, 0, 5)).toBe(0));

	test('いちばん下（隙間 = 件数）へ落とせる', () => {
		// **両端を名指しできること。**ここが数えられないと、末尾へ動かせません。
		expect(dropTarget(0, 5, 5)).toBe(4);
	});

	test('自分の両隣へ落としても、動かない', () => {
		// **同じ内容でファイルを書き直さない。**
		expect(dropTarget(2, 2, 5)).toBeNull();
		expect(dropTarget(2, 3, 5)).toBeNull();
	});

	test('範囲の外は断る', () => {
		expect(dropTarget(0, -1, 5)).toBeNull();
		expect(dropTarget(0, 6, 5)).toBeNull();
		expect(dropTarget(9, 1, 5)).toBeNull();
	});
});

describe('並べ替えた一覧', () => {
	test('下から上へ', () => {
		expect(reorder(['a', 'b', 'c', 'd'], 2, 0)).toEqual(['c', 'a', 'b', 'd']);
	});

	test('上から下へ', () => {
		expect(reorder(['a', 'b', 'c', 'd'], 0, 3)).toEqual(['b', 'c', 'd', 'a']);
	});

	test('元の配列を書き換えない', () => {
		const before = ['a', 'b', 'c'];
		reorder(before, 0, 2);
		expect(before).toEqual(['a', 'b', 'c']);
	});
});

describe('指の位置から隙間を決める', () => {
	// 行の高さ 40、4 行。中点は 20 / 60 / 100 / 140。
	const midpoints = [20, 60, 100, 140];

	test('いちばん上より上なら、隙間 0', () => {
		expect(gapForPointer(5, midpoints)).toBe(0);
	});

	test('いちばん下より下なら、隙間 = 件数', () => {
		// **末尾へ動かせること。**ここが数えられないと最後尾へ行けません。
		expect(gapForPointer(200, midpoints)).toBe(4);
	});

	test('中点を越えたところで、次の隙間へ移る', () => {
		expect(gapForPointer(59, midpoints)).toBe(1);
		expect(gapForPointer(61, midpoints)).toBe(2);
	});

	test('行が 1 件も無ければ、隙間は 0 しかない', () => {
		expect(gapForPointer(123, [])).toBe(0);
	});
});
