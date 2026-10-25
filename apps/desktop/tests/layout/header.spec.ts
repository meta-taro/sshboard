import { expect, test, type Page } from '@playwright/test';

/**
 * **窓の枠（ヘッダー）が、どの幅でも使えること。**
 *
 * 0.1.27 で、Mac の写真（1100×700）を綺麗にするために製品名とタブを縮めない形にしたら、
 * **Windows では右上の × が 2 段目の左端（高さ 13px）へ落ち、MCP の釦が印だけになりました。**
 * Windows の窓は表示倍率のせいで、論理の幅が 1240px を下回るのが普通です。
 *
 * ここでは**狭い幅から広い幅まで**、日本語と英語で、次が崩れないことを測ります。
 */

/** 860 は 1366×768・150% の窓に近い幅。1240 は狭い窓の規則が切り替わる境目。 */
const WIDTHS = [860, 1000, 1100, 1239, 1240, 1400] as const;
const LOCALES = ['ja', 'en'] as const;

/** 窓の操作の釦の大きさ。これより小さいと、押せないのと同じです。 */
const MIN_CONTROL_PX = 28;
/** 右上の角からのずれの許し。帯の外殻の余白ぶん。 */
const CORNER_SLACK_PX = 24;

async function open(page: Page, locale: string, width: number): Promise<void> {
	await page.addInitScript((code) => {
		try {
			localStorage.setItem('sshboard-locale', code);
		} catch {
			// 保存できない環境でも、既定の言語で測れれば足りる。
		}
	}, locale);
	await page.setViewportSize({ width, height: 700 });
	await page.goto('/');
	await page.locator('header').waitFor();
}

for (const locale of LOCALES) {
	for (const width of WIDTHS) {
		test.describe(`${locale} · ${width}px`, () => {
			test.beforeEach(async ({ page }) => open(page, locale, width));

			test('keeps the close button in the top right corner, big enough to press', async ({
				page
			}) => {
				const close = page.locator('.wc.close');
				await expect(close).toBeVisible();
				const box = await close.boundingBox();
				expect(box, '× が描かれていない').not.toBeNull();
				if (!box) return;
				expect(box.height, '× が潰れている').toBeGreaterThanOrEqual(MIN_CONTROL_PX);
				expect(box.width, '× が潰れている').toBeGreaterThanOrEqual(MIN_CONTROL_PX);
				expect(box.y, '× が上の段に居ない').toBeLessThan(CORNER_SLACK_PX);
				expect(width - (box.x + box.width), '× が右端に居ない').toBeLessThan(CORNER_SLACK_PX);
			});

			test('keeps minimise and maximise beside it', async ({ page }) => {
				const controls = page.locator('.window-controls .wc');
				await expect(controls).toHaveCount(3);
				const boxes = await controls.evaluateAll((els) =>
					els.map((el) => el.getBoundingClientRect().toJSON() as DOMRect)
				);
				const tops = new Set(boxes.map((b) => Math.round(b.top)));
				expect(tops.size, '─ □ × が同じ段に並んでいない').toBe(1);
			});

			test('labels the MCP button in words, not only an icon', async ({ page }) => {
				const mcp = page.locator('button.mcp');
				await expect(mcp).toBeVisible();
				// **見えている字で測る。**隠した字（display: none）も textContent には入るので、
				// それで測ると印だけの釦でも通ってしまいます。
				await expect(mcp).toContainText('MCP', { useInnerText: true });
			});

			test('does not let anything slide under the window controls', async ({ page }) => {
				const overlaps = await page.evaluate(() => {
					const controls = document.querySelector('.window-controls')!.getBoundingClientRect();
					const others = document.querySelectorAll('header > *:not(.window-controls)');
					return [...others]
						.map((el) => ({ cls: el.className, r: el.getBoundingClientRect() }))
						.filter(
							({ r }) =>
								r.width > 0 &&
								r.left < controls.right &&
								r.right > controls.left &&
								r.top < controls.bottom &&
								r.bottom > controls.top
						)
						.map(({ cls }) => cls);
				});
				expect(overlaps, '窓の操作の下に潜った札がある').toEqual([]);
			});

			test('does not cut the product name or push the page sideways', async ({ page }) => {
				const brand = page.locator('.brand');
				await expect(brand).toBeVisible();
				const cut = await brand.evaluate((el) => el.scrollWidth > el.clientWidth + 1);
				expect(cut, '製品名が切れている').toBe(false);
				const sideways = await page.evaluate(
					() => document.documentElement.scrollWidth > window.innerWidth + 1
				);
				expect(sideways, '横スクロールが出ている').toBe(false);
			});
		});
	}
}
