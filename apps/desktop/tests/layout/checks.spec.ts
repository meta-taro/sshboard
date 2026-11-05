import { expect, test, type Page } from '@playwright/test';

import { calledWith, fakeTauri, opened, setReply } from './fake-tauri';
import { lookFindings } from './look';

/**
 * **点検（D72）**。人が書いた点検を並べ、［回す］で回し、良し悪しと前回の値を見せる。
 *
 * > ヘルスチェックスクリプトみたいなのを流せる仕組みが欲しい。一度作ったら、
 * > そのコマンドを順に実行して値をエージェントと人がチェックできる（オーナー・2026-11-04）
 */
const ON = 'web-01';
const PATH = '/home/someone/.config/sshboard/checks.toml';

const DISK = { kind: 'good' };
function report(value: string, previous?: string) {
	return {
		id: 'morning',
		description: '毎朝の点検',
		connection: ON,
		steps: [
			{
				readonly: 'disk',
				title: 'ディスク使用率',
				judged: { verdict: DISK, value },
				...(previous ? { previous: { verdict: DISK, value: previous } } : {})
			},
			{
				readonly: 'nginx',
				title: 'nginx',
				judged: { verdict: { kind: 'bad', why: '「active」が出ていません' }, value: 'inactive' }
			},
			{
				readonly: 'not-listed',
				title: '許可リストに無いもの',
				judged: { verdict: { kind: 'unknown', why: '許可リストに在りません' }, value: '' }
			}
		],
		good: 1,
		bad: 1,
		unknown: 1
	};
}

function overview(last: unknown = null) {
	return {
		path: PATH,
		checks: [
			{
				id: 'morning',
				description: '毎朝の点検',
				steps: ['ディスク使用率', 'nginx', '許可リストに無いもの'],
				last
			}
		]
	};
}

async function openChecks(page: Page, replies: Record<string, unknown>): Promise<void> {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened(ON, 'Web server')], active: ON },
		...replies
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await page.locator('button.brand').click();
	await page.getByRole('button', { name: '点検', exact: true }).click();
}

test('with nothing written yet, the dialog says so and names the file to write', async ({ page }) => {
	await openChecks(page, { checks_overview: { path: PATH, checks: [] } });

	const dialog = page.getByRole('dialog', { name: '点検' });
	await expect(dialog).toContainText('まだ 1 つも書かれていません');
	await expect(dialog.getByText(PATH)).toBeVisible();
	// **置き場所は伏せられる所**（利用者名が入る・禁止事項 4）。
	await expect(dialog.getByText(PATH)).toHaveAttribute('data-secret', '');
});

test('run goes through check_run and shows good, bad and unknown with their reasons', async ({ page }) => {
	await openChecks(page, { checks_overview: overview(), check_run: report('42%') });
	const dialog = page.getByRole('dialog', { name: '点検' });
	await expect(dialog).toContainText('まだ回していません');

	await dialog.getByRole('button', { name: '回す' }).click();

	const sent = await calledWith(page, 'check_run');
	expect(sent.map((c) => c.args)).toEqual([{ id: 'morning' }]);
	await expect(dialog).toContainText('良し 1 ・ 悪し 1 ・ 測れず 1');
	const rows = dialog.locator('tbody tr');
	await expect(rows).toHaveCount(3);
	await expect(rows.nth(0)).toContainText('42%');
	await expect(rows.nth(1)).toContainText('「active」が出ていません');
	await expect(rows.nth(2)).toContainText('許可リストに在りません');
	// **値はサーバーから来た字**（ホスト名が混ざりうる）。伏せられる所に置く。
	await expect(rows.nth(0).locator('[data-secret]').first()).toHaveText('42%');
});

test('the previous value sits beside the new one', async ({ page }) => {
	await openChecks(page, { checks_overview: overview(report('41%')), check_run: report('42%', '41%') });
	const dialog = page.getByRole('dialog', { name: '点検' });
	// **AI が回した結果も、開いた時点で出ている。**
	await expect(dialog.locator('tbody tr').first()).toContainText('41%');

	await setReply(page, 'check_run', report('42%', '41%'));
	await dialog.getByRole('button', { name: '回す' }).click();

	const first = dialog.locator('tbody tr').first();
	await expect(first).toContainText('42%');
	await expect(first).toContainText('41%');
});

for (const scheme of ['light', 'dark'] as const) {
	test(`the dialog has no look problems (${scheme})`, async ({ page }) => {
		await page.emulateMedia({ colorScheme: scheme });
		await openChecks(page, { checks_overview: overview(report('42%', '41%')) });
		await expect(page.getByRole('dialog', { name: '点検' }).locator('tbody tr')).toHaveCount(3);

		expect(await lookFindings(page)).toEqual([]);
	});
}
