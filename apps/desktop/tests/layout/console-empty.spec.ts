import { expect, test } from '@playwright/test';

import { calledWith, fakeTauri, opened } from './fake-tauri';

/**
 * **「上の［端末を開く］を押してください」と言うのに、その釦が無かった**（Windows 実機・2026-10-25）。
 *
 * 別の接続で端末を開いていると（人が握っている）、［端末を開く］は出ない作りでした。
 * そこで端末の無い接続へ切り替えると、**空の面は「上の［端末を開く］を押して」と言い、
 * 上には ［AI に渡す］［止める］しか無い。**人はどの釦か分かりません。
 *
 * **言った釦が、言った所に在ること**を測ります。
 */
const VIEWING = 'web-01';
const ELSEWHERE = 'db-01';

test.beforeEach(async ({ page }) => {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: {
			open: [opened(VIEWING, 'Web server'), opened(ELSEWHERE, 'Database')],
			active: VIEWING
		},
		console_holder: { holder: 'human', connection: ELSEWHERE },
		console_list: [{ id: 1, connection: ELSEWHERE, holder: 'human' }],
		connections_list: [],
		console_open_another: 2
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await page.getByRole('button', { name: '端末', exact: true }).click();
});

test('says there is no console on this connection yet', async ({ page }) => {
	await expect(page.getByText('まだ端末が 1 枚も開いていません')).toBeVisible();
});

test('puts the button it talks about where the person can press it', async ({ page }) => {
	await expect(page.locator('.console-panes').getByRole('button', { name: '端末を開く' })).toBeVisible();
});

test('opens a console on the connection being looked at, not the other one', async ({ page }) => {
	await page.locator('.console-panes').getByRole('button', { name: '端末を開く' }).click();
	const sent = await calledWith(page, 'console_open_another');
	expect(sent).toHaveLength(1);
	expect(sent[0].args).toMatchObject({ connectionId: VIEWING });
});

test('does not point at a button that is not there', async ({ page }) => {
	// 釦を面の中に置いたので、「上の」とは言わない。
	await expect(page.getByText('上の［端末を開く］')).toHaveCount(0);
});

test('keeps the button inside the pane, big enough to press', async ({ page }) => {
	const pane = await page.locator('.console-panes').boundingBox();
	const button = await page
		.locator('.console-panes')
		.getByRole('button', { name: '端末を開く' })
		.boundingBox();
	expect(pane && button, '面か釦が描かれていない').toBeTruthy();
	if (!pane || !button) return;
	expect(button.height).toBeGreaterThanOrEqual(28);
	expect(button.y).toBeGreaterThanOrEqual(pane.y);
	expect(button.y + button.height).toBeLessThanOrEqual(pane.y + pane.height);
});
