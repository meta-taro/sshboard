/**
 * **帯の書き出し**（案・4 枚目）。
 *
 * ## ここで見るいちばん大事なこと
 *
 * **画面と同じものが出ること。**
 *
 * 書き出したものが画面と違うと、**どちらが本当か分からなくなります。**
 * そして**伏せているときは伏せたまま出す** ——
 * 「見えているものと、書き出したものが違う」を作りません。
 */
import { describe, expect, test } from 'vitest';

import type { BandLine } from './band';
import { bandExportText, defaultBandName, EXPORT_HEADER } from './band-export';

const AT = new Date('2026-10-23T12:00:00.000Z');

const LINES: BandLine[] = [
	{ seq: 1, tag: '[Human]', text: 'connect web-01', rendered: '[Human] connect web-01' },
	{ seq: 2, tag: '[AI]', text: 'read /srv/app/x', rendered: '[AI]    read /srv/app/x' }
];

describe('帯の書き出し', () => {
	test('writes the lines as the screen shows them', () => {
		// **並べ替えも間引きもしません。**
		const text = bandExportText(LINES, { masked: false, at: AT });
		expect(text).toContain('[Human] connect web-01');
		expect(text).toContain('[AI]    read /srv/app/x');
		expect(text.indexOf('connect')).toBeLessThan(text.indexOf('read /srv'));
	});

	test('warns, in the file, that it carries where you connect to', () => {
		// **貼る前に読まれる所に置きます**（禁止事項 4）。
		const text = bandExportText(LINES, { masked: false, at: AT });
		expect(text).toContain(EXPORT_HEADER);
		expect(text).toContain('公開の場');
	});

	test('carries no markdown, because a .txt renders nothing', () => {
		// **画面の文言と同じ決まり**（`messages.test.ts`）。
		// 素のテキストに `**` を書いても、描く物は何もありません。
		for (const masked of [false, true]) {
			const text = bandExportText(LINES, { masked, at: AT });
			expect(text, `masked=${masked}`).not.toContain('**');
		}
	});

	test('says how many lines it carries', () => {
		expect(bandExportText(LINES, { masked: false, at: AT })).toContain('2 行');
	});

	test('hides the content when the screen is hidden', () => {
		// **画面と同じものが出ること。**
		const text = bandExportText(LINES, { masked: true, at: AT });
		expect(text).not.toContain('web-01');
		expect(text).not.toContain('/srv/app/x');
	});

	test('still says who did what when hidden', () => {
		// **誰が動かしたかは残します。**そこが帯の値打ちです。
		const text = bandExportText(LINES, { masked: true, at: AT });
		expect(text).toContain('[Human]');
		expect(text).toContain('[AI]');
	});

	test('says in the file that it was written while hidden', () => {
		// 後から読む人が「●」を壊れたファイルだと思わないように。
		expect(bandExportText(LINES, { masked: true, at: AT })).toContain('伏せた状態');
	});

	test('hides the length too', () => {
		// **字数から推測されないように。**
		const short: BandLine[] = [{ seq: 1, tag: '[AI]', text: 'x', rendered: '[AI] x' }];
		const long: BandLine[] = [
			{ seq: 1, tag: '[AI]', text: 'a'.repeat(60), rendered: `[AI] ${'a'.repeat(60)}` }
		];
		const a = bandExportText(short, { masked: true, at: AT }).split('\n').at(-2);
		const b = bandExportText(long, { masked: true, at: AT }).split('\n').at(-2);
		expect(a).toBe(b);
	});

	test('writes something readable even with nothing in the band', () => {
		const text = bandExportText([], { masked: false, at: AT });
		expect(text).toContain('0 行');
		expect(text.endsWith('\n')).toBe(true);
	});

	test('puts the time in the name, so a second save does not overwrite', () => {
		const name = defaultBandName(AT);
		expect(name).toMatch(/^sshboard-band-2026-10-23T12-00-00\.txt$/);
		expect(defaultBandName(new Date('2026-10-23T12:00:01Z'))).not.toBe(name);
	});
});
