// @vitest-environment jsdom
/**
 * **帯。**人と AI の操作が時間順に 1 本で並ぶ所。
 *
 * **これがこの道具の要です**（PRD §4-1「見えないことが最大の危険」）。
 *
 * ## ソースを読む検査から、描いて確かめる検査へ
 *
 * これまで `+page.svelte` のソース文字列を見て
 * 「`filterBand` を呼んでいるか」「`bandLines` を描いているか」を数えていました。
 * **部品に出したので、実際に描いて確かめられます** ——
 * そちらが強い検査です（書き方を変えても、**出るものが同じなら通る**）。
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, test } from 'vitest';

import { AI_TAG, HUMAN_TAG, type BandLine } from '$lib/band';
import { i18n } from '$lib/i18n/i18n.svelte';

import BandPanel from './BandPanel.svelte';

function line(seq: number, tag: string, text: string): BandLine {
	return { seq, tag, text, rendered: `${tag} ${text}` };
}

const LINES: BandLine[] = [
	line(1, HUMAN_TAG, 'connect web-01'),
	line(2, AI_TAG, 'session_status'),
	line(3, HUMAN_TAG, '$ sudo certbot renew --dry-run'),
	line(4, AI_TAG, 'read_log')
];

afterEach(() => cleanup());

/** 絞りの札を押す。 */
async function pick(key: 'band.who.all' | 'band.who.ai' | 'band.who.human') {
	await fireEvent.click(screen.getByRole('button', { name: i18n.t(key) }));
}

describe('帯', () => {
	test('shows everything it was given, in order', () => {
		// **既定で分けない。**1 本に並んでいることが要。
		render(BandPanel, { lines: LINES });

		const shown = screen.getAllByRole('listitem').map((li) => li.textContent);
		expect(shown).toEqual(LINES.map((l) => l.rendered));
	});

	test('says plainly when nothing has happened yet', () => {
		render(BandPanel, { lines: [] });
		expect(screen.getByText(i18n.t('band.empty'))).toBeTruthy();
	});

	test('does not offer a filter when there is nothing to filter', () => {
		render(BandPanel, { lines: [] });
		expect(screen.queryByRole('button', { name: i18n.t('band.who.ai') })).toBeNull();
	});
});

describe('誰の行かで絞る', () => {
	test('keeps only what the AI did', async () => {
		render(BandPanel, { lines: LINES });
		await pick('band.who.ai');

		const shown = screen.getAllByRole('listitem').map((li) => li.textContent);
		expect(shown).toEqual([LINES[1].rendered, LINES[3].rendered]);
	});

	test('keeps only what the person did', async () => {
		render(BandPanel, { lines: LINES });
		await pick('band.who.human');

		const shown = screen.getAllByRole('listitem').map((li) => li.textContent);
		expect(shown).toEqual([LINES[0].rendered, LINES[2].rendered]);
	});

	test('comes back to everything', async () => {
		render(BandPanel, { lines: LINES });
		await pick('band.who.ai');
		await pick('band.who.all');

		expect(screen.getAllByRole('listitem').length).toBe(LINES.length);
	});

	test('says which one is chosen, not only by colour', async () => {
		// **人と AI の色は明るさがほぼ同じ**（差 9.5 / 255）。
		// 色が見えない人にも「いまどれで見ているか」が届くこと。
		render(BandPanel, { lines: LINES });
		await pick('band.who.ai');

		const ai = screen.getByRole('button', { name: i18n.t('band.who.ai') });
		expect(ai.getAttribute('aria-pressed')).toBe('true');
		const all = screen.getByRole('button', { name: i18n.t('band.who.all') });
		expect(all.getAttribute('aria-pressed')).toBe('false');
	});

	test('does not call an empty filter result "nothing has happened"', async () => {
		// **行は在るのに絞りで消えている**のを、壊れたと読ませない。
		render(BandPanel, { lines: [line(1, HUMAN_TAG, 'connect')] });
		await pick('band.who.ai');

		expect(screen.getByText(i18n.t('band.filtered.empty'))).toBeTruthy();
		expect(screen.queryByText(i18n.t('band.empty'))).toBeNull();
	});

	test('marks the AI rows so they can be told apart', () => {
		// 行そのものにも印を付けます（絞らずに読むときのため）。
		render(BandPanel, { lines: LINES });

		const items = screen.getAllByRole('listitem');
		expect(items[0].classList.contains('ai')).toBe(false);
		expect(items[1].classList.contains('ai')).toBe(true);
	});
});
