import { expect, test, type Locator } from '@playwright/test';

import { stage } from './scenes';

/**
 * **チェックボックスの四辺が、どの表示倍率でも同じ濃さ**（Issue #28・2026-11-02 の実機）。
 *
 * > ちぇっくぼっくすがうえうすくてきもちわるい ／ うえがほそいせんになってるだろが
 *
 * Windows・125% で「左右はくっきり、上は薄い灰色、下は 2 段ににじむ」（100% では四辺同じ）。
 * 縦の位置が端数（294.55px など）で、**OS が描く素のチェックボックス**の線が
 * 画素の境目にまたがって薄まっていた。ここでは倍率 125% / 150% で撮り、
 * 四辺それぞれのいちばん濃い所を比べる。
 */
async function edgeDarkness(box: Locator): Promise<number[]> {
	const shot = await box.screenshot({ scale: 'device' });
	return box.page().evaluate(async (bytes: number[]) => {
		const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
		const bitmap = await createImageBitmap(blob);
		const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
		const ctx = canvas.getContext('2d')!;
		ctx.drawImage(bitmap, 0, 0);
		const { data, width, height } = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
		const lum = (x: number, y: number) => {
			const i = (y * width + x) * 4;
			return (data[i] + data[i + 1] + data[i + 2]) / 3;
		};
		const band = (xs: number[], ys: number[]) => {
			let darkest = 255;
			for (const x of xs) for (const y of ys) darkest = Math.min(darkest, lum(x, y));
			return darkest;
		};
		const third = (n: number) => Math.max(1, Math.floor(n / 3));
		const range = (a: number, b: number) => Array.from({ length: b - a }, (_, k) => a + k);
		const midX = range(third(width), width - third(width));
		const midY = range(third(height), height - third(height));
		return [
			band(midX, range(0, third(height))), // 上
			band(midX, range(height - third(height), height)), // 下
			band(range(0, third(width)), midY), // 左
			band(range(width - third(width), width), midY) // 右
		];
	}, Array.from(shot));
}

for (const dsf of [1.25, 1.5]) {
	test.describe(`表示倍率 ${dsf * 100}%`, () => {
		test.use({ deviceScaleFactor: dsf });

		test('draws all four edges of a checkbox equally dark', async ({ page }) => {
			await stage(page, 'fresh', { locale: 'ja', theme: 'light', width: 1100, height: 700 });
			const box = page.locator('input.tick').first();
			await box.waitFor();
			const [top, bottom, left, right] = await edgeDarkness(box);
			const sides = [top, bottom, left, right];
			expect(
				Math.max(...sides) - Math.min(...sides),
				`四辺の濃さが揃っていない（上 ${top}・下 ${bottom}・左 ${left}・右 ${right}）`
			).toBeLessThanOrEqual(12);
		});
	});
}
