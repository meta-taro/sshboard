/**
 * 再起動を押すと何が切れるか。**ここだけは他のアプリと重さが違います。**
 *
 * エディタや DB クライアントの再起動と違い、sshboard は
 * **生きている SSH・開いている端末・MCP が共有している同じ 1 本**を落とします。
 * 数えられなければ札に書けないので、数える所を単体で確かめます。
 */
import { describe, expect, test } from 'vitest';

import { cutMessageKey, cutSummary } from './restart-cost';

describe('切れるものを数える', () => {
	test('何も繋がっておらず端末も無ければ、何も言わない', () => {
		// **余計な脅しを出さない。**毎回出る警告は、読まれない警告になる。
		expect(cutSummary({ sessions: 0, terminalOpen: false })).toBeNull();
	});

	test('1 本繋がっていれば、その 1 本を数える', () => {
		expect(cutSummary({ sessions: 1, terminalOpen: false })).toEqual({
			sessions: 1,
			terminal: false
		});
	});

	test('複数本でも数える', () => {
		expect(cutSummary({ sessions: 3, terminalOpen: false })).toEqual({
			sessions: 3,
			terminal: false
		});
	});

	test('端末だけ開いていても言う', () => {
		expect(cutSummary({ sessions: 0, terminalOpen: true })).toEqual({
			sessions: 0,
			terminal: true
		});
	});

	test('接続と端末の両方', () => {
		expect(cutSummary({ sessions: 2, terminalOpen: true })).toEqual({
			sessions: 2,
			terminal: true
		});
	});

	test('数えられていない値は 0 として扱う', () => {
		// **信じられない数を人へ見せない。**「-1 本が切れます」は出さない。
		expect(cutSummary({ sessions: -1, terminalOpen: false })).toBeNull();
		expect(cutSummary({ sessions: Number.NaN, terminalOpen: false })).toBeNull();
		expect(cutSummary({ sessions: 1.5, terminalOpen: false })).toBeNull();
	});
});

describe('どの文を出すか', () => {
	test('接続だけなら、接続の文', () => {
		expect(cutMessageKey({ sessions: 2, terminal: false })).toBe('update.cuts');
	});

	test('接続と端末なら、両方の文', () => {
		// 1 つの文に足し算で書くと「3 本と 1 つが切れます」のような不自然な訳になる。
		expect(cutMessageKey({ sessions: 2, terminal: true })).toBe('update.cuts.both');
	});

	test('端末だけなら、端末の文', () => {
		expect(cutMessageKey({ sessions: 0, terminal: true })).toBe('update.cuts.terminal');
	});
});
