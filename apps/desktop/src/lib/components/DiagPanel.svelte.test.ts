// @vitest-environment jsdom
/**
 * **診断の面。**繋がらなかったときに「なぜ」を読む所。
 *
 * **握り潰さないための面です** —— 失敗が画面に出ないと、
 * 人は「壊れている」としか言えません。
 *
 * ## ここで見る 3 つ
 *
 * 1. **`hint`（どうすれば通るか）が出ること。**これが落ちると、
 *    「失敗しました」だけが並ぶ面になります。
 * 2. **重さ（`level`）が色だけでなく階級として付くこと。**
 * 3. **接続の名前に伏せる印が付くこと**（D26）——
 *    この製品は画面そのものに接続先が写り込みます。
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, test } from 'vitest';

import type { DiagEvent } from '$lib/diagnostics';
import { i18n } from '$lib/i18n/i18n.svelte';

import DiagPanel from './DiagPanel.svelte';

const EVENTS: DiagEvent[] = [
	{ seq: 1, atMs: 1200, level: 'info', stage: 'Connect', message: '繋ぎに行きます' },
	{
		seq: 2,
		atMs: 3400,
		level: 'error',
		stage: 'Auth',
		connection: 'web-01',
		message: '認証が失敗しました（鍵が受け付けられませんでした）',
		hint: 'ssh-agent に鍵が載っているか確かめてください'
	},
	{ seq: 3, atMs: 5000, level: 'warn', stage: 'Sftp', message: '一覧が空でした' }
];

afterEach(() => cleanup());

describe('診断の面', () => {
	test('says plainly when there is nothing to read', () => {
		render(DiagPanel, { found: [], onrefresh: () => {} });
		expect(screen.getByText(i18n.t('diag.empty'))).toBeTruthy();
	});

	test('shows every event it was given', () => {
		render(DiagPanel, { found: EVENTS, onrefresh: () => {} });
		expect(screen.getAllByRole('listitem').length).toBe(EVENTS.length);
	});

	test('shows how to get past it, not only that it failed', () => {
		// **これが落ちると「失敗しました」だけが並ぶ面になります。**
		render(DiagPanel, { found: EVENTS, onrefresh: () => {} });
		expect(screen.getByText(/ssh-agent に鍵が載っているか/)).toBeTruthy();
	});

	test('marks how heavy each line is, as a class not only a colour', () => {
		// **色だけだと、色の見分けが難しい人に届きません。**
		render(DiagPanel, { found: EVENTS, onrefresh: () => {} });
		const items = screen.getAllByRole('listitem');
		expect(items[0].classList.contains('info')).toBe(true);
		expect(items[1].classList.contains('error')).toBe(true);
		expect(items[2].classList.contains('warn')).toBe(true);
	});

	test('turns the elapsed milliseconds into seconds', () => {
		// `atMs` は**起動からの経過**で、日付ではありません。
		render(DiagPanel, { found: EVENTS, onrefresh: () => {} });
		expect(screen.getByText('3.4s')).toBeTruthy();
	});

	test('lets the connection name be hidden', () => {
		// **この製品は画面そのものに接続先が写り込みます**（D26 / 禁止事項 4）。
		// 伏せる印が無い所は、画面を伏せても残ってしまいます。
		render(DiagPanel, { found: EVENTS, onrefresh: () => {} });
		// 接続ごとのタブにも名前が出るので、**出ている所すべてに**印があること。
		const shown = screen.getAllByText('web-01');
		expect(shown.length).toBeGreaterThan(0);
		for (const who of shown) expect(who.hasAttribute('data-secret')).toBe(true);
	});

	test('asks for a fresh read when the person presses it', () => {
		let asked = 0;
		render(DiagPanel, { found: EVENTS, onrefresh: () => (asked += 1) });

		fireEvent.click(screen.getByRole('button', { name: i18n.t('diag.refresh') }));
		expect(asked).toBe(1);
	});
});

describe('接続ごとのタブ（オーナー・2026-10-26）', () => {
	const MIXED: DiagEvent[] = [
		{ seq: 3, atMs: 3000, level: 'info', stage: 'exec', connection: 'db-01', message: 'db の行' },
		{ seq: 2, atMs: 2000, level: 'info', stage: 'exec', message: '接続の無い行' },
		{ seq: 1, atMs: 1000, level: 'info', stage: 'auth', connection: 'web-01', message: 'web の行' }
	];

	test('offers "all" and one tab per connection', () => {
		render(DiagPanel, { found: MIXED, onrefresh: () => {} });
		const tabs = screen.getAllByRole('tab');
		expect(tabs.map((tab) => tab.textContent?.trim())).toEqual([
			i18n.t('diag.all'),
			'db-01',
			'web-01'
		]);
	});

	test('starts on "all", showing every line', () => {
		render(DiagPanel, { found: MIXED, onrefresh: () => {} });
		expect(screen.getByRole('tab', { name: i18n.t('diag.all') }).getAttribute('aria-selected')).toBe(
			'true'
		);
		expect(screen.getAllByRole('listitem')).toHaveLength(3);
	});

	test('shows only that connection’s lines when its tab is pressed', async () => {
		render(DiagPanel, { found: MIXED, onrefresh: () => {} });
		await fireEvent.click(screen.getByRole('tab', { name: 'db-01' }));
		const shown = screen.getAllByRole('listitem').map((li) => li.textContent);
		expect(shown).toHaveLength(1);
		expect(shown[0]).toContain('db の行');
	});

	test('marks the connection names on the tabs as hideable', () => {
		// **接続の名前は伏せられる所**（D26・禁止事項 4）。
		render(DiagPanel, { found: MIXED, onrefresh: () => {} });
		const tab = screen.getByRole('tab', { name: 'web-01' });
		expect(tab.querySelector('[data-secret]')).not.toBeNull();
	});

	test('does not offer tabs when no line names a connection', () => {
		render(DiagPanel, { found: [EVENTS[0]], onrefresh: () => {} });
		expect(screen.queryAllByRole('tab')).toHaveLength(0);
	});
});
