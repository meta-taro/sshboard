import { expect, test, type Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened } from './fake-tauri';

/**
 * **窓そのものはスクロールしない。各面が自分の中でスクロールする**（オーナー・2026-10-26）。
 *
 * > UI として、ソフト全体のスクロールを廃止させてください。おのおの内部スクロールで。
 *
 * 端末の面を開くと、窓の右端に**アプリ全体の縦スクロールバー**が出ていました。
 * 窓ごと動くと、端末と帯の上下がずれ、ヘッダー（窓を掴む所・× の在る所）まで画面の外へ流れます。
 *
 * 中身を十分に溢れさせて（帯 40 行・接続 30 件）、どの面・どの大きさでも測ります。
 */
const VIEWS = ['接続', 'ファイル', '端末', '履歴', '診断'] as const;
const SIZES = [
	{ width: 1100, height: 700 },
	{ width: 1100, height: 560 },
	{ width: 860, height: 600 },
	{ width: 1400, height: 900 }
] as const;

const ON = 'web-01';

function connection(n: number) {
	return {
		id: `demo-${n}`,
		name: `Demo ${n}`,
		host: 'example.invalid',
		port: 22,
		user: 'demo',
		write_roots: []
	};
}

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
		connections_list: Array.from({ length: 30 }, (_, n) => connection(n)),
		console_holder: { holder: 'human', connection: ON },
		console_list: [{ id: 1, connection: ON, holder: 'human' }],
		diagnostics_recent: Array.from({ length: 60 }, (_, n) => ({
			seq: n,
			atMs: n * 100,
			level: 'info',
			stage: 'exec',
			connection: ON,
			message: `記録 ${n}`
		}))
	});
}

async function fillBand(page: Page): Promise<void> {
	for (let n = 0; n < 40; n += 1) {
		await emitEvent(page, 'band://line', {
			seq: n,
			tag: n % 2 ? '[AI]' : '[Human]',
			text: `line ${n}`,
			rendered: `${n % 2 ? '[AI]' : '[Human]'} line ${n}`
		});
	}
}

for (const size of SIZES) {
	for (const view of VIEWS) {
		test(`${view} · ${size.width}×${size.height}: the window itself does not scroll`, async ({
			page
		}) => {
			await crowded(page);
			await page.setViewportSize(size);
			await page.goto('/');
			await fillBand(page);
			await page.getByRole('button', { name: view, exact: true }).click();
			await page.waitForTimeout(300);

			const overflow = await page.evaluate(() => ({
				doc: document.documentElement.scrollHeight - window.innerHeight,
				body: document.body.scrollHeight - window.innerHeight
			}));
			expect(overflow.doc, '窓全体に縦スクロールが出ている').toBeLessThanOrEqual(1);
			expect(overflow.body, 'body に縦スクロールが出ている').toBeLessThanOrEqual(1);

			// **ヘッダーは常に窓の中**（掴む所・× を流さない）。
			const header = await page.locator('header').boundingBox();
			expect(header?.y ?? -1, 'ヘッダーが窓の外へ流れた').toBeGreaterThanOrEqual(0);
		});
	}
}

test('the band beside the terminal scrolls inside itself, so its last line can be reached', async ({
	page
}) => {
	await crowded(page);
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
	await fillBand(page);
	await page.getByRole('button', { name: '端末', exact: true }).click();

	const band = page.locator('.three > section.band');
	const inside = await band.evaluate((el) => ({
		overflowY: getComputedStyle(el).overflowY,
		scrolls: el.scrollHeight > el.clientHeight
	}));
	expect(inside.overflowY, '帯が自分でスクロールしない（切れているだけ）').toMatch(/auto|scroll/);
	expect(inside.scrolls, '帯が溢れていない（試験の前提が崩れた）').toBe(true);

	await band.evaluate((el) => el.scrollTo(0, el.scrollHeight));
	// **溢れた行が、窓の外ではなく帯の中で読めること。**
	await expect(band.locator('li').last()).toBeInViewport();
});
