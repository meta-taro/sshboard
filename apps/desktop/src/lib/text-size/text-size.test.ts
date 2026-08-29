/**
 * 文字サイズ。**端で止まること**と、**保存の鍵が `app.html` と揃っていること。**
 *
 * 鍵がずれると「保存はされるが、次の起動で標準に戻る」という、
 * 使っていて理由の分からない壊れ方になります。
 */
import { describe, expect, test } from 'vitest';

import { isTextSize, TEXT_SIZES, TEXT_SIZE_STORAGE_KEY } from './text-size.svelte';

describe('isTextSize', () => {
	test('accepts only the four sizes', () => {
		for (const size of TEXT_SIZES) expect(isTextSize(size)).toBe(true);
	});

	test('rejects anything else, including null and a stale value', () => {
		// 古い設定や、人が手で書き換えた値をそのまま当てない。
		expect(isTextSize(null)).toBe(false);
		expect(isTextSize('')).toBe(false);
		expect(isTextSize('huge')).toBe(false);
		expect(isTextSize('120%')).toBe(false);
	});
});

describe('the sizes themselves', () => {
	test('are ordered from smallest to largest', () => {
		// **この並びが「1 段上げる / 下げる」の順序**なので、崩れると操作が逆になる。
		expect([...TEXT_SIZES]).toEqual(['small', 'normal', 'large', 'xlarge']);
	});

	test('include a normal size, which is the default', () => {
		expect(TEXT_SIZES).toContain('normal');
	});
});

describe('the storage key', () => {
	test('matches the one app.html applies before the first paint', () => {
		// **片方だけ変えると、最初の一瞬だけ別の大きさになる。**
		// app.html 側は文字列べた書きなので、ここで突き合わせる。
		expect(TEXT_SIZE_STORAGE_KEY).toBe('sshboard-text-size');
	});
});

describe('stepping', () => {
	// 端の扱いは純粋な計算。**同じ規則をここで確かめる。**
	const step = (from: string, delta: 1 | -1) => {
		const at = TEXT_SIZES.indexOf(from as (typeof TEXT_SIZES)[number]);
		return TEXT_SIZES[Math.min(TEXT_SIZES.length - 1, Math.max(0, at + delta))];
	};

	test('stops at the ends instead of wrapping around', () => {
		// 一周させると、**行き過ぎたことに気づけない。**
		expect(step('small', -1)).toBe('small');
		expect(step('xlarge', 1)).toBe('xlarge');
	});

	test('moves one step at a time', () => {
		expect(step('small', 1)).toBe('normal');
		expect(step('normal', 1)).toBe('large');
		expect(step('large', -1)).toBe('normal');
	});
});
