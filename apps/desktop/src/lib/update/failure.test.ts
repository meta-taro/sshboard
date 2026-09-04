/**
 * 更新の失敗を、人の次の一手で分ける。
 *
 * いまは `String(error)` がそのまま札へ流れ込みます。訳されないうえ、
 * **中身に URL やパスが入りえます。**この製品は画面に写り込むものを気にする
 * （CLAUDE.md 禁止事項 4）のに、ここだけ素通しでした。
 *
 * 分けるのは 3 つ。**繋がらなかった / 署名が合わなかった / 分からない。**
 * 人の次の一手が違うのは、この 3 つだけです。
 */
import { describe, expect, test } from 'vitest';

import { classify, messageKeyFor } from './failure';

describe('繋がらなかった', () => {
	test('取りに行けなかった類', () => {
		expect(classify('TypeError: Failed to fetch')).toBe('network');
		expect(classify('Network Error')).toBe('network');
		expect(classify('error sending request for url')).toBe('network');
		expect(classify('operation timed out')).toBe('network');
		expect(classify('dns error: failed to lookup address')).toBe('network');
	});
});

describe('署名が合わなかった', () => {
	test('確かめられなかった類', () => {
		expect(classify('Signature verification failed')).toBe('signature');
		expect(classify('minisign: invalid signature')).toBe('signature');
		expect(classify('untrusted comment mismatch')).toBe('signature');
	});

	test('繋がった上で署名が合わない場合は、署名の側を優先する', () => {
		// **人の一手が違う。**回線を疑うのではなく、配布元を疑うべき場面。
		expect(classify('failed to fetch signature: untrusted')).toBe('signature');
	});
});

describe('分からないもの', () => {
	test('分からないと言う', () => {
		expect(classify('')).toBe('unknown');
		expect(classify('something else entirely')).toBe('unknown');
	});
});

describe('画面へ渡すもの', () => {
	test('3 分類それぞれに 1 本の鍵がある', () => {
		expect(messageKeyFor('Failed to fetch')).toBe('update.failed.network');
		expect(messageKeyFor('invalid signature')).toBe('update.failed.signature');
		expect(messageKeyFor('???')).toBe('update.failed.unknown');
	});

	test('生の文字列が鍵に混ざらない', () => {
		// **画面へ流すのは鍵だけ。**URL・パス・トークンの類は 1 文字も持ち出さない。
		const raw = 'https://example.invalid/latest.json?token=abcdef /Users/someone/x';
		expect(messageKeyFor(raw)).not.toContain('example.invalid');
		expect(messageKeyFor(raw)).not.toContain('/Users/');
	});
});
