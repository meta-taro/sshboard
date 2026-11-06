import { expect, test } from '@playwright/test';

import { calledWith, emitEvent, fakeTauri, opened } from './fake-tauri';

/**
 * **AI が出した操作の案も、よそで打った Esc では消えない**（セキュリティレビュー LOW-2・2026-11-06）。
 *
 * 案は人が端末で作業している最中に出ます。窓全体で Esc を拾うと、
 * vim を抜けるための Esc で、読む前に案が消えます。
 */
test('escape typed outside the operation proposal does not dismiss it', async ({ page }) => {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened('web-01', 'Web server')], active: 'web-01' },
		operation_proposal: null
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await emitEvent(page, 'operation://proposal', {
		id: 'reload-nginx',
		run: 'sudo systemctl reload nginx',
		description: '設定を読み直す',
		maxPerHour: 3
	});
	const dialog = page.getByRole('dialog');
	await expect(dialog).toBeVisible();

	await page.locator('body').press('Escape');
	await expect(dialog).toBeVisible();
	expect(await calledWith(page, 'operation_dismiss')).toHaveLength(0);

	await dialog.getByRole('button').first().focus();
	await page.keyboard.press('Escape');
	await expect(dialog).toBeHidden();
	expect(await calledWith(page, 'operation_dismiss')).toHaveLength(1);
});
