import { expect, test, type Page } from '@playwright/test';

import { calledWith, emitEvent, fakeTauri, opened } from './fake-tauri';
import { lookFindings } from './look';

/**
 * **AI が出した点検の案**（D72 の次の段）。AI は書かない。保存するのは画面の人。
 */
const PROPOSAL = {
	id: 'daily',
	description: '毎朝の点検',
	step: [
		{ readonly: 'disk-root', label: 'ディスク使用率', below: 90 },
		{ readonly: 'nginx-active', contains: 'active' },
		{ readonly: 'dmesg', absent: 'Out of memory' },
		{ readonly: 'uptime' }
	]
};

async function shown(page: Page): Promise<void> {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened('web-01', 'Web server')], active: 'web-01' },
		check_proposal: null
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await emitEvent(page, 'check://proposal', PROPOSAL);
}

test('the proposal shows every step with its criterion before anything is saved', async ({ page }) => {
	await shown(page);
	const dialog = page.getByRole('dialog', { name: '点検の案' });

	await expect(dialog).toContainText('daily');
	await expect(dialog).toContainText('毎朝の点検');
	const rows = dialog.locator('tbody tr');
	await expect(rows).toHaveCount(4);
	await expect(rows.nth(0)).toContainText('ディスク使用率');
	await expect(rows.nth(0)).toContainText('90 未満');
	await expect(rows.nth(1)).toContainText('「active」が出る');
	await expect(rows.nth(2)).toContainText('「Out of memory」が出ない');
	await expect(rows.nth(3)).toContainText('正常に終わる');
	// **保存しても回らない**ことを、押す前に言う。
	await expect(dialog).toContainText('保存しても回りません');
});

test('save goes through the screen-only command and the dialog closes', async ({ page }) => {
	await shown(page);
	const dialog = page.getByRole('dialog', { name: '点検の案' });

	await dialog.getByRole('button', { name: '保存する' }).click();

	expect(await calledWith(page, 'check_proposal_save')).toHaveLength(1);
	await expect(dialog).toBeHidden();
});

test('declining writes nothing and the dialog closes', async ({ page }) => {
	await shown(page);
	const dialog = page.getByRole('dialog', { name: '点検の案' });

	await dialog.getByRole('button', { name: '断る' }).click();

	expect(await calledWith(page, 'check_proposal_dismiss')).toHaveLength(1);
	expect(await calledWith(page, 'check_proposal_save')).toHaveLength(0);
	await expect(dialog).toBeHidden();
});

for (const scheme of ['light', 'dark'] as const) {
	test(`the proposal has no look problems (${scheme})`, async ({ page }) => {
		await page.emulateMedia({ colorScheme: scheme });
		await shown(page);
		await expect(page.getByRole('dialog', { name: '点検の案' }).locator('tbody tr')).toHaveCount(4);

		expect(await lookFindings(page)).toEqual([]);
	});
}
