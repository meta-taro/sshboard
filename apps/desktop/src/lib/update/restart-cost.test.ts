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
			terminal: false,
			mcp: false
				});
	});

	test('複数本でも数える', () => {
		expect(cutSummary({ sessions: 3, terminalOpen: false })).toEqual({
			sessions: 3,
			terminal: false,
			mcp: false
				});
	});

	test('端末だけ開いていても言う', () => {
		expect(cutSummary({ sessions: 0, terminalOpen: true })).toEqual({
			sessions: 0,
			terminal: true,
			mcp: false
				});
	});

	test('接続と端末の両方', () => {
		expect(cutSummary({ sessions: 2, terminalOpen: true })).toEqual({
			sessions: 2,
			terminal: true,
			mcp: false
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
		expect(cutMessageKey({ sessions: 2, terminal: false, mcp: false })).toBe('update.cuts');
	});

	test('接続と端末なら、両方の文', () => {
		// 1 つの文に足し算で書くと「3 本と 1 つが切れます」のような不自然な訳になる。
		expect(cutMessageKey({ sessions: 2, terminal: true, mcp: false })).toBe('update.cuts.both');
	});

	test('端末だけなら、端末の文', () => {
		expect(cutMessageKey({ sessions: 0, terminal: true, mcp: false })).toBe('update.cuts.terminal');
	});
});

describe('AI の目も道連れになること（UI 案・2026-10-21）', () => {
	/**
	 * **GUI と MCP は同じ 1 本の SSH を共有します**（CLAUDE.md 禁止事項 3）。
	 *
	 * つまり立ち上げ直すと、**AI が見ているものも一緒に切れます。**
	 * それまで数えていたのは人の側だけ（接続と端末）で、
	 * **AI の分は名乗っていませんでした** ——
	 * 案はそこを「AI — MCP 接続 1 本」と明示しています。
	 *
	 * **知らないまま押させない**のがこの関数の役目なので、
	 * 人の分だけ数えるのは役目を半分しか果たしていません。
	 */
	test('counts the AI as something that gets cut', () => {
		expect(cutSummary({ sessions: 0, terminalOpen: false, mcpOpen: true })).toEqual({
			sessions: 0,
			terminal: false,
			mcp: true
		});
	});

	test('says so even when the person has nothing open', () => {
		// **人は何も開いていないのに AI は読んでいる**ことが起きます
		// （AI が宛先を動かした直後など）。そこで `null` を返すと、
		// 「何も切れません」と言って AI を切ることになります。
		const cost = cutSummary({ sessions: 0, terminalOpen: false, mcpOpen: true });
		expect(cost).not.toBeNull();
		expect(cutMessageKey(cost!)).toBe('update.cuts.ai');
	});

	test('still returns null when truly nothing is open', () => {
		expect(cutSummary({ sessions: 0, terminalOpen: false, mcpOpen: false })).toBeNull();
	});

	test('mentions the AI alongside the person', () => {
		// 人の分と AI の分は**別の文**にします。足し算で 1 文に押し込むと
		// 訳が不自然になる（この関数が最初からそう決めている）。
		const both = cutSummary({ sessions: 2, terminalOpen: true, mcpOpen: true });
		expect(both).toEqual({ sessions: 2, terminal: true, mcp: true });
		expect(cutMessageKey(both!)).toBe('update.cuts.both');
	});

	test('treats a missing answer as the AI not being there', () => {
		// 呼ぶ側が古いとき。**居ないものを「切れます」と言わない。**
		expect(cutSummary({ sessions: 1, terminalOpen: false })).toEqual({
			sessions: 1,
			terminal: false,
			mcp: false
		});
	});
});
