/**
 * 変更内容を札へ収める。
 *
 * **札の中にスクロール箱を作らない。**先頭だけを出し、残りは数で示して、
 * 押したときにその場で伸ばします。比べた 2 つのアプリはどちらも箱の中で
 * 文の途中が切れていて、**切れていること自体に気づけない**形でした。
 *
 * 本文は配布元から来る任意の文字列なので、**長さも行数もここで切ります。**
 */
import { describe, expect, test } from 'vitest';

import { MAX_NOTE_LINE, MAX_NOTE_LINES, summarise } from './notes';

describe('畳む', () => {
	test('何も無ければ何も出さない', () => {
		expect(summarise('')).toEqual({ head: [], rest: 0, lines: [] });
		expect(summarise('   \n\n  \n')).toEqual({ head: [], rest: 0, lines: [] });
	});

	test('3 行までなら、そのまま全部出る', () => {
		const summary = summarise('直した\n足した\n消した');
		expect(summary.head).toEqual(['直した', '足した', '消した']);
		expect(summary.rest).toBe(0);
	});

	test('4 行目からは「あと何件」で数える', () => {
		const summary = summarise(['a', 'b', 'c', 'd', 'e'].join('\n'));
		expect(summary.head).toEqual(['a', 'b', 'c']);
		expect(summary.rest).toBe(2);
		expect(summary.lines).toHaveLength(5);
	});

	test('何行出すかは変えられる', () => {
		expect(summarise('a\nb\nc\nd', 2).head).toEqual(['a', 'b']);
		expect(summarise('a\nb\nc\nd', 2).rest).toBe(2);
	});

	test('空行は落とす', () => {
		// 空行を数に入れると「あと 5 件」と言って 2 件しか出ない、が起きる。
		expect(summarise('a\n\n\nb').lines).toEqual(['a', 'b']);
	});

	test('CRLF でも同じ結果になる', () => {
		expect(summarise('a\r\nb\r\nc').head).toEqual(['a', 'b', 'c']);
	});
});

describe('記号を剥がす', () => {
	test('Markdown の見出しと箇条書きを落とす', () => {
		// **札は Markdown を描きません。**記号がそのまま出ると、ただのゴミに見える。
		const summary = summarise('## 修正\n- 直した\n* もう 1 つ\n+ 3 つ目\n1. 4 つ目\n・5 つ目', 10);
		expect(summary.lines).toEqual(['修正', '直した', 'もう 1 つ', '3 つ目', '4 つ目', '5 つ目']);
	});

	test('文中のハイフンは残す', () => {
		expect(summarise('ssh-agent を使う').lines).toEqual(['ssh-agent を使う']);
	});
});

describe('長さを切る', () => {
	test('長すぎる 1 行は切って、切ったことが分かるようにする', () => {
		const long = 'あ'.repeat(MAX_NOTE_LINE + 40);
		const [line] = summarise(long).head;
		expect(line).toHaveLength(MAX_NOTE_LINE + 1);
		expect(line.endsWith('…')).toBe(true);
	});

	test('ちょうどの長さは切らない', () => {
		const exact = 'あ'.repeat(MAX_NOTE_LINE);
		expect(summarise(exact).head[0]).toBe(exact);
	});

	test('全文にも行数の上限がある', () => {
		// 配布元が何行送ってきても、画面が伸び続けない。
		const many = Array.from({ length: MAX_NOTE_LINES + 50 }, (_, at) => `行 ${at}`).join('\n');
		expect(summarise(many).lines).toHaveLength(MAX_NOTE_LINES);
	});
});
