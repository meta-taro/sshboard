import { expect, test } from '@playwright/test';

import { lookFindings } from './look';
import { stage, type Scene } from './scenes';

/**
 * **どの面にも、同じ見栄えの物差しを当てる**（オーナー・2026-10-26）。
 *
 * > 見栄え崩れは初めてではないので。見栄えの TDD チェックも追加してください。
 *
 * 面 5 つ × 場面 2 つ（何も無い・中身が多い）× 大きさ 3 つ × 明暗 × 日英 × 2 エンジン。
 */
const VIEWS = {
	ja: ['接続', 'ファイル', '端末', '履歴', '診断'],
	en: ['Connections', 'Files', 'Console', 'Activity', 'Log']
} as const;
const SIZES = [
	{ width: 860, height: 600 },
	{ width: 1100, height: 700 },
	{ width: 1400, height: 900 }
] as const;
const SCENES: Scene[] = ['fresh', 'busy'];

for (const locale of ['ja', 'en'] as const) {
	for (const theme of ['light', 'dark']) {
		for (const scene of SCENES) {
			for (const size of SIZES) {
				test(`${locale} · ${theme} · ${scene} · ${size.width}×${size.height}`, async ({ page }) => {
					await stage(page, scene, { locale, theme, ...size });
					const problems: string[] = [];
					for (const [n, view] of VIEWS[locale].entries()) {
						await page.locator('nav.tabs button').nth(n).click();
						await page.waitForTimeout(150);
						for (const finding of await lookFindings(page)) problems.push(`[${view}] ${finding}`);
					}
					expect(problems, problems.join('\n')).toEqual([]);
				});
			}
		}
	}
}
