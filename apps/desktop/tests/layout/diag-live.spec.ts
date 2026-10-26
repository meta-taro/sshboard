import { expect, test } from '@playwright/test';

import { fakeTauri, setReply } from './fake-tauri';

/**
 * **診断の面は、開いている間ずっと新しいまま**（Windows 実機・2026-10-26）。
 *
 * MCP が固まった最中に人が［診断］を見たら、**約 400 秒前の行で止まっていました。**
 * 端末を開いた・握りを取り返した・AI に渡した、のどれも見えない。
 * 実行体はそれらを書いていますが、**面は開いた瞬間に 1 回読むだけ**だったので、
 * 開いたままの面には古い一覧が残り続けました。**固まった瞬間こそ見に行く所**なのに。
 */
function line(seq: number, message: string) {
	return { seq, atMs: seq * 1000, level: 'info', stage: 'exec', connection: 'web-01', message };
}

test.beforeEach(async ({ page }) => {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		diagnostics_recent: [line(1, '繋がりました')],
		connections_list: []
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await page.getByRole('button', { name: '診断', exact: true }).click();
	await expect(page.getByText('繋がりました')).toBeVisible();
});

test('shows a line written after the view was opened, without pressing anything', async ({
	page
}) => {
	await setReply(page, 'diagnostics_recent', [
		line(2, '人が握りを AI へ渡しました'),
		line(1, '繋がりました')
	]);
	// **何も押さずに**出てくること。読み直しの釦を押させない。
	await expect(page.getByText('人が握りを AI へ渡しました')).toBeVisible({ timeout: 5000 });
});
