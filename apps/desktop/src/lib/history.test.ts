import { describe, expect, test } from 'vitest';

import {
	back,
	canBack,
	canForward,
	createHistory,
	current,
	forward,
	visit,
	type History
} from './history';

describe('たどった道を覚える', () => {
	test('作った直後は、そこに居るだけで戻る先が無い', () => {
		const h = createHistory('/home/me');
		expect(current(h)).toBe('/home/me');
		expect(canBack(h)).toBe(false);
		expect(canForward(h)).toBe(false);
	});

	test('進むと戻れるようになる', () => {
		const h = visit(createHistory('/a'), '/a/b');
		expect(current(h)).toBe('/a/b');
		expect(canBack(h)).toBe(true);
	});

	test('戻ると、進めるようになる', () => {
		const h = back(visit(createHistory('/a'), '/a/b'));
		expect(current(h)).toBe('/a');
		expect(canBack(h)).toBe(false);
		expect(canForward(h)).toBe(true);
		expect(current(forward(h))).toBe('/a/b');
	});

	test('戻ったあとに別の所へ行くと、先の分は捨てる', () => {
		// **ブラウザと同じ。**残すと「進む」がどこへ行くか読めなくなる。
		let h: History = visit(createHistory('/a'), '/a/b');
		h = back(h);
		h = visit(h, '/a/c');
		expect(current(h)).toBe('/a/c');
		expect(canForward(h)).toBe(false);
		expect(current(back(h))).toBe('/a');
	});

	test('同じ所へ続けて行っても増やさない', () => {
		// **再読み込みで履歴が伸びない。**伸びると「戻る」を何度押しても同じ場所。
		let h = createHistory('/a');
		h = visit(h, '/a');
		h = visit(h, '/a');
		expect(canBack(h)).toBe(false);
	});

	test('端で押しても落ちない', () => {
		const h = createHistory('/a');
		expect(current(back(h))).toBe('/a');
		expect(current(forward(h))).toBe('/a');
	});

	test('たどり続けても、覚える数に上限がある', () => {
		// **際限なく持たない。**長く使うほど太る一方になる。
		let h = createHistory('/0');
		for (let i = 1; i <= 300; i++) h = visit(h, `/${i}`);
		expect(current(h)).toBe('/300');
		let count = 0;
		let walk = h;
		while (canBack(walk)) {
			walk = back(walk);
			count++;
		}
		expect(count).toBeLessThanOrEqual(200);
		expect(count).toBeGreaterThan(0);
	});

	test('元の履歴を書き換えない', () => {
		// immutability（coding-style）。**共有している側が黙って変わらない。**
		const first = createHistory('/a');
		const second = visit(first, '/a/b');
		expect(current(first)).toBe('/a');
		expect(current(second)).toBe('/a/b');
	});
});
