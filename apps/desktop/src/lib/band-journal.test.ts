/**
 * **過去の帯**（Phase 2「監査ログの永続化」・2026-11-25）。
 *
 * 控えは UTC の日付で 1 日 1 本に分かれ、1 件ごとに UNIX 時刻（ミリ秒）を持ちます。
 * 画面では**その人の時計の時刻**で並べます —— 「15 時ごろ AI が何をしたか」を探す人に、
 * UTC の 6 時を見せても探せません。
 */
import { describe, expect, test } from 'vitest';

import { journalLine, type JournalEntry } from './band-journal';

const entry = (overrides: Partial<JournalEntry> = {}): JournalEntry => ({
	atMs: Date.UTC(2026, 10, 25, 6, 7, 8),
	seq: 3,
	tag: '[AI]',
	text: 'connect web',
	...overrides
});

describe('journalLine', () => {
	test('puts the local clock time in front of the band line', () => {
		const at = new Date(Date.UTC(2026, 10, 25, 6, 7, 8));
		const hh = String(at.getHours()).padStart(2, '0');
		const mm = String(at.getMinutes()).padStart(2, '0');
		const ss = String(at.getSeconds()).padStart(2, '0');
		expect(journalLine(entry())).toBe(`${hh}:${mm}:${ss} [AI]    connect web`);
	});

	test('lines up human lines with AI lines, as the live band does', () => {
		const ai = journalLine(entry());
		const human = journalLine(entry({ tag: '[Human]', text: 'connect web' }));
		expect(ai.indexOf('connect')).toBe(human.indexOf('connect'));
	});
});
