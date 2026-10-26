import { expect, test, type Page } from '@playwright/test';

import { fakeTauri } from './fake-tauri';

/**
 * **［ファイル］の「接続先」の枠**（Windows 実機・0.1.30・2026-10-26・カムスタ経由）。
 *
 * 1. 見出し「接続先」が単独のカードとして浮き、一覧の箱の上辺に乗り上げて角がずれていた
 * 2. 接続の札が 2 段に折り返すと、段の間が約 70px 空き、箱が中身よりずっと高かった
 * 3. 見出しと一覧の箱の間に隙間と段差があった
 *
 * 期待: 見出しは一覧と同じ 1 つの箱の上端に入る。段の間隔は札 1 つ分＋通常の隙間。
 * 箱の高さは中身に合う。
 */
const NAMES = [
	'kingdom 001',
	'kingdom 002',
	'L5 web',
	'L5 db',
	'dk006',
	'dk008',
	'dk009',
	'batch',
	'stage',
	'mail'
];

async function openFilesWithNothingConnected(page: Page, width: number, theme: string) {
	await page.addInitScript(
		([locale, mode]) => {
			try {
				localStorage.setItem('sshboard-locale', locale);
				localStorage.setItem('sshboard-theme', mode);
			} catch {
				// 保存できなくても既定で測れる。
			}
		},
		['ja', theme]
	);
	await fakeTauri(page, {
		session_status: { open: [], active: null },
		transfers_recent: [],
		diagnostics_recent: [],
		console_list: [],
		connections_list: NAMES.map((name, n) => ({
			id: `c${n}`,
			name,
			host: 'example.invalid',
			port: 22,
			user: 'demo',
			write_roots: []
		}))
	});
	await page.setViewportSize({ width, height: 760 });
	await page.goto('/');
	await page.locator('nav.tabs button', { hasText: 'ファイル' }).click();
	await page.locator('.picker').waitFor();
}

for (const width of [860, 1100, 1300]) {
	for (const theme of ['light', 'dark']) {
		test.describe(`${width}px · ${theme}`, () => {
			test.beforeEach(async ({ page }) => openFilesWithNothingConnected(page, width, theme));

			test('puts the heading inside the same box as the list', async ({ page }) => {
				const box = await page.locator('.picker').boundingBox();
				const heading = await page.getByText('接続先', { exact: true }).boundingBox();
				expect(box && heading, '箱か見出しが描かれていない').toBeTruthy();
				if (!box || !heading) return;
				expect(heading.y, '見出しが箱の上にはみ出ている').toBeGreaterThanOrEqual(box.y);
				expect(heading.x, '見出しが箱の左にはみ出ている').toBeGreaterThanOrEqual(box.x);
				expect(heading.y + heading.height).toBeLessThanOrEqual(box.y + box.height);
				// **見出しだけの箱を、別に浮かせない。**
				await expect(page.locator('.files > .bar')).toHaveCount(0);
			});

			test('keeps every part of the picker inside its box', async ({ page }) => {
				const outside = await page.locator('.picker').evaluate((box) => {
					const frame = box.getBoundingClientRect();
					return [...box.children]
						.map((child) => ({ cls: child.className, r: child.getBoundingClientRect() }))
						.filter(
							({ r }) =>
								r.left < frame.left - 0.5 ||
								r.right > frame.right + 0.5 ||
								r.top < frame.top - 0.5 ||
								r.bottom > frame.bottom + 0.5
						)
						.map(({ cls }) => cls);
				});
				expect(outside, '箱からはみ出た部品がある').toEqual([]);
			});

			test('wraps the connections without a tall gap between rows', async ({ page }) => {
				const tops = await page.locator('.picker .pick').evaluateAll((picks) => ({
					tops: [...new Set(picks.map((p) => Math.round(p.getBoundingClientRect().top)))].sort(
						(a, b) => a - b
					),
					height: Math.max(...picks.map((p) => p.getBoundingClientRect().height))
				}));
				for (let n = 1; n < tops.tops.length; n += 1) {
					expect(
						tops.tops[n] - tops.tops[n - 1],
						'段の間が札 1 つ分＋隙間より広い'
					).toBeLessThanOrEqual(tops.height + 12);
				}
			});

			test('sizes the list to its contents', async ({ page }) => {
				const slack = await page.locator('.picker .core').evaluate((list) => {
					const picks = [...list.querySelectorAll('.pick')].map((p) => p.getBoundingClientRect());
					const used = Math.max(...picks.map((r) => r.bottom)) - Math.min(...picks.map((r) => r.top));
					return list.getBoundingClientRect().height - used;
				});
				expect(slack, '一覧の箱が中身よりずっと高い').toBeLessThanOrEqual(28);
			});
		});
	}
}
