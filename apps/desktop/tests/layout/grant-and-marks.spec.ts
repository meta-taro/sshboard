import { expect, test, type Page } from '@playwright/test';

import { fakeTauri, opened } from './fake-tauri';

/**
 * **0.1.48 で見出しと端末の面に足した 2 つ**を、描いて測ります（2026-11-16）。
 *
 * 1. 端末の面の左の並びに出す**印の色**。色の棒が、その接続の札の中に見えていること
 * 2. 出ている**期限付きの許可**の帯。足したことで見出しがはみ出したり、
 *    窓の操作（─ □ ×）の下に潜ったりしないこと（0.1.27 で × が消えた退行と同じ測り方）
 */
const COLORED = 'web-01';
const PLAIN = 'db-01';
const WIDTHS = [860, 1100, 1400] as const;
const LOCALES = ['ja', 'en'] as const;

async function open(page: Page, locale: string, width: number): Promise<void> {
	await page.addInitScript((code) => {
		try {
			localStorage.setItem('sshboard-locale', code);
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	}, locale);
	await fakeTauri(page, {
		session_status: {
			open: [{ ...opened(COLORED, 'Web server'), color: 'red' }, opened(PLAIN, 'Database')],
			active: COLORED
		},
		console_holder: { holder: null, connection: null },
		console_list: [],
		connections_list: [],
		console_grant_state: { scope: 'new', connection: null, remainingSecs: 3 * 60 * 60 }
	});
	await page.setViewportSize({ width, height: 700 });
	await page.goto('/');
	await page.locator('header').waitFor();
}

/** 許可の枠を開いて閉じる。**開いたときに実行体から読み直す**ので、帯が出ます。 */
async function showGrant(page: Page): Promise<void> {
	await page.locator('button.brand').click();
	await page.locator('.view-menu').getByRole('button').filter({ hasText: /許可|permission/i }).last().click();
	await page.getByRole('dialog').getByRole('button').first().waitFor();
	await page.keyboard.press('Escape');
	await expect(page.locator('.grant-chip')).toBeVisible();
}

for (const locale of LOCALES) {
	for (const width of WIDTHS) {
		test.describe(`${locale} · ${width}px`, () => {
			test.beforeEach(async ({ page }) => open(page, locale, width));

			test('keeps the header inside the window with the grant chip shown', async ({ page }) => {
				await showGrant(page);
				const sideways = await page.evaluate(
					() => document.documentElement.scrollWidth > window.innerWidth + 1
				);
				expect(sideways, '許可の帯で横スクロールが出た').toBe(false);
				const overlaps = await page.evaluate(() => {
					const controls = document.querySelector('.window-controls')?.getBoundingClientRect();
					const chip = document.querySelector('.grant-chip')?.getBoundingClientRect();
					if (!controls || !chip) return null;
					return (
						chip.left < controls.right &&
						chip.right > controls.left &&
						chip.top < controls.bottom &&
						chip.bottom > controls.top
					);
				});
				expect(overlaps, '許可の帯が窓の操作の下に潜った').toBe(false);
			});
		});
	}
}

test.describe('印の色（端末の面）', () => {
	test.beforeEach(async ({ page }) => {
		await open(page, 'ja', 1100);
		await page.getByRole('button', { name: '端末', exact: true }).click();
	});

	test('shows the colour bar inside the coloured connection, and none on the plain one', async ({
		page
	}) => {
		const picks = page.locator('.rail.vertical .pick');
		await expect(picks).toHaveCount(2);
		const bar = picks.nth(0).locator('.mark-bar');
		await expect(bar).toBeVisible();
		const painted = await bar.evaluate((el) => getComputedStyle(el).backgroundColor);
		expect(painted, '色の棒に色が付いていない').not.toMatch(/rgba\(0, 0, 0, 0\)|transparent/);
		const inside = await picks.nth(0).evaluate((pick) => {
			const p = pick.getBoundingClientRect();
			const b = pick.querySelector('.mark-bar')!.getBoundingClientRect();
			return b.left >= p.left && b.right <= p.right && b.top >= p.top && b.bottom <= p.bottom;
		});
		expect(inside, '色の棒が札からはみ出している').toBe(true);
		await expect(picks.nth(1).locator('.mark-bar')).toHaveCount(0);
	});

	test('does not cut the connection name to make room for the bar', async ({ page }) => {
		const name = page.locator('.rail.vertical .pick').nth(0).locator('.who');
		const cut = await name.evaluate((el) => el.scrollWidth > el.clientWidth + 1);
		expect(cut, '色の棒のせいで名前が切れた').toBe(false);
	});
});
