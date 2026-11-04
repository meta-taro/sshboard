import { expect, test, type Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened } from './fake-tauri';

/**
 * **スクロールは 1 段だけ**（Windows 11・100%・窓 1240×738・0.1.35・2026-11-04）。
 *
 * > 二重スクロールでてます、なおしてくだしあ。
 *
 * 端末の面で、右の帯が窓の下で切れ、**3 列の入れ物そのものにも縦スクロールが出ていました。**
 * 帯は自分の中でスクロールするのに、その外側も動く —— 2 段のスクロールです。
 * 外側を回すと帯の下端が窓の外へ逃げ、**AI が何をしたかの最後の行が見えなくなります**
 * （PRD §4-1「見えないことが最大の危険」）。
 *
 * 3 列が横に並びきる広さなら、**入れ物は動かず、各列が自分の中でだけスクロールする。**
 * 3 列に並ぶのは端末の面だけです（ほかの面は `no-page-scroll.spec.ts` が窓ごとの動きを見ています）。
 */
const ON = 'web-01';
const SIZES = [
	{ width: 1240, height: 738 },
	{ width: 1224, height: 699 },
	{ width: 1100, height: 700 },
	{ width: 1400, height: 900 }
] as const;

async function crowded(page: Page): Promise<void> {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened(ON, 'Web server')], active: ON },
		console_holder: { holder: 'human', connection: ON },
		console_list: [{ id: 1, connection: ON, holder: 'human' }]
	});
}

async function fillBand(page: Page): Promise<void> {
	for (let n = 0; n < 60; n += 1) {
		await emitEvent(page, 'band://line', {
			seq: n,
			tag: n % 2 ? '[AI]' : '[Human]',
			text: `line ${n}`,
			rendered: `${n % 2 ? '[AI]' : '[Human]'} line ${n}`
		});
	}
}

for (const size of SIZES) {
	test(`端末 · ${size.width}×${size.height}: only the columns scroll, never the row that holds them`, async ({
		page
	}) => {
		await crowded(page);
		await page.setViewportSize(size);
		await page.goto('/');
		await fillBand(page);
		await page.getByRole('button', { name: '端末', exact: true }).click();
		await page.waitForTimeout(300);

		const row = await page.locator('.three').evaluate((el) => {
			const tops = [...el.children].map((c) => Math.round(c.getBoundingClientRect().top));
			return { oneRow: new Set(tops).size === 1, over: el.scrollHeight - el.clientHeight };
		});
		// 狭くて列が下へ落ちたときだけ、入れ物のスクロールは許されている（落ちない大きさだけを測る）。
		expect(row.oneRow, '試験の前提が崩れた：列が折り返した').toBe(true);
		expect(row.over, '3 列の入れ物にもスクロールが出ている（二重スクロール）').toBeLessThanOrEqual(1);

		// **帯の下端は窓の中**（最後の行へ帯の中だけで届く）。
		const box = await page.locator('.three > section.band').boundingBox();
		const bottom = (box?.y ?? 0) + (box?.height ?? 0);
		expect(bottom, '帯が窓の下で切れている').toBeLessThanOrEqual(size.height + 1);
	});
}
