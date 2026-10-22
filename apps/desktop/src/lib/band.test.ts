/**
 * 帯の行の扱い。**ここは「誰がやったか」の出どころです。**
 *
 * 帯は人と AI の操作が時間順に 1 本で並ぶ所で、**それがこの道具の要**です
 * （PRD §4-1「見えないことが最大の危険」）。
 *
 * 絞り込み（UI 案・2026-10-21）は**混ぜたものを分けて見せるだけ**で、
 * 混ぜるのをやめません。既定は「全部」。
 *
 * **絞りが間違うと、AI の操作が人のものとして見えます。**
 * それは「何も出ていない」より悪い —— 人は自分がやったと思い込みます。
 */
import { describe, expect, test } from 'vitest';

import { AI_TAG, appendLine, filterBand, HUMAN_TAG, MAX_VISIBLE_LINES, type BandLine } from './band';

/** 行を 1 つ作る。 */
function line(seq: number, tag: string, text: string): BandLine {
	return { seq, tag, text, rendered: `${tag} ${text}` };
}

const LINES: BandLine[] = [
	line(1, HUMAN_TAG, 'connect'),
	line(2, AI_TAG, 'session_status'),
	line(3, HUMAN_TAG, '$ sudo certbot renew --dry-run'),
	line(4, AI_TAG, 'read_log'),
	// **知らない札。**将来足したものが、いまここに無い名前で来る場合。
	line(5, '[System]', 'updated to 0.1.25')
];

describe('誰の行かで絞る', () => {
	test('shows everything by default', () => {
		// **1 本に並んでいることが要。**既定で分けない。
		expect(filterBand(LINES, 'all').map((l) => l.seq)).toEqual([1, 2, 3, 4, 5]);
	});

	test('shows only what the AI did', () => {
		expect(filterBand(LINES, 'ai').map((l) => l.seq)).toEqual([2, 4]);
	});

	test('shows only what the person did', () => {
		expect(filterBand(LINES, 'human').map((l) => l.seq)).toEqual([1, 3]);
	});

	test('keeps the order it was given', () => {
		// 時間順が崩れると、**どちらが先に触ったか**が読めなくなります。
		const got = filterBand(LINES, 'all');
		expect(got.map((l) => l.seq)).toEqual([...got].sort((a, b) => a.seq - b.seq).map((l) => l.seq));
	});

	test('does not call an unknown tag a person', () => {
		// **知らない札を人だと決めつけない。**
		// 決めつけると、**AI の操作を人のものとして見せる**ことになります。
		// 出さない方がまし —— 「全部」で見れば在ります。
		expect(filterBand(LINES, 'human').some((l) => l.tag === '[System]')).toBe(false);
		expect(filterBand(LINES, 'ai').some((l) => l.tag === '[System]')).toBe(false);
		expect(filterBand(LINES, 'all').some((l) => l.tag === '[System]')).toBe(true);
	});

	test('does not touch the array it was given', () => {
		const before = [...LINES];
		filterBand(LINES, 'ai');
		expect(LINES).toEqual(before);
	});

	test('returns a new array even when nothing is filtered out', () => {
		// 同じ配列を返すと、**呼んだ側が書き換えたときに元が壊れます。**
		expect(filterBand(LINES, 'all')).not.toBe(LINES);
	});

	test('survives an empty band', () => {
		expect(filterBand([], 'ai')).toEqual([]);
	});
});

describe('行を足す', () => {
	test('does not touch the array it was given', () => {
		const lines = [line(1, HUMAN_TAG, 'connect')];
		const before = [...lines];
		appendLine(lines, line(2, AI_TAG, 'session_status'));
		expect(lines).toEqual(before);
	});

	test('stops the band from growing without a bound', () => {
		// `tail -f` を流すと、**上限が無ければ画面が落ちるまで溜まります。**
		let lines: BandLine[] = [];
		for (let i = 0; i < MAX_VISIBLE_LINES + 50; i += 1) {
			lines = appendLine(lines, line(i, AI_TAG, 'x'));
		}
		expect(lines.length).toBe(MAX_VISIBLE_LINES);
	});

	test('drops the oldest, not the newest', () => {
		// **新しい方を捨てると、いま起きていることが見えません。**
		let lines: BandLine[] = [];
		for (let i = 0; i < MAX_VISIBLE_LINES + 3; i += 1) {
			lines = appendLine(lines, line(i, AI_TAG, 'x'));
		}
		expect(lines[lines.length - 1].seq).toBe(MAX_VISIBLE_LINES + 2);
		expect(lines[0].seq).toBe(3);
	});
});
