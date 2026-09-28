/**
 * **`[object Object]` を出さない**（実運用の指摘・Issue #26）。
 *
 * > 上部の帯に **`[object Object]`** だけが出ています。前後に文字はありません。
 *
 * **この字だけで 2 日使いました。**人にも、受け取った AI にも、何も伝わりません。
 */
import { describe, expect, test } from 'vitest';

import { asConnectFailure, readableFailure } from './connect-failure';

describe('繋ぎ損ねた理由', () => {
	test('**構造を持った失敗を、そうと見抜く**', () => {
		expect(asConnectFailure({ kind: 'passphraseNeeded' })?.kind).toBe('passphraseNeeded');
		expect(
			asConnectFailure({ kind: 'untrusted', algorithm: 'ssh-ed25519', fingerprint: 'SHA256:x', expected: null })
				?.kind
		).toBe('untrusted');
		expect(asConnectFailure({ nope: 1 })).toBeNull();
		expect(asConnectFailure(null)).toBeNull();
		expect(asConnectFailure('ただの文字')).toBeNull();
	});

	test('**どんなものを渡しても [object Object] にしない**', () => {
		const anything: unknown[] = [
			{ kind: 'passphraseNeeded' },
			{ kind: 'untrusted', algorithm: 'ssh-ed25519', fingerprint: 'SHA256:x', expected: null },
			{ kind: 'other', message: '繋がりません' },
			{ 知らない: '形' },
			new Error('普通の失敗'),
			'ただの文字',
			42,
			null,
			undefined
		];
		for (const one of anything) {
			expect(readableFailure(one), `${JSON.stringify(one)} で潰れています`).not.toBe(
				'[object Object]'
			);
		}
	});

	test('`other` は、そのままの文言を出す', () => {
		expect(readableFailure({ kind: 'other', message: '繋がりません: 接続が拒否されました' })).toBe(
			'繋がりません: 接続が拒否されました'
		);
	});

	test('知らない形でも、中身を見せる', () => {
		// **「分からない」で終わらせない。**追える字を残します。
		expect(readableFailure({ 知らない: '形' })).toContain('知らない');
	});
});
