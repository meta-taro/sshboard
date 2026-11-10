import { expect, test, type Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened } from './fake-tauri';

/**
 * **AI が端末を握っているときも、スクロールは 1 段だけ**（Windows 11・0.1.39・2026-11-10）。
 *
 * > ２重スクロールになって操作性悪化しました。
 *
 * 11-04 の試験（`one-scroll.spec.ts`）は「人が握る端末 1 本・柵なし」だけを測っていました。
 * 実機では **AI が握り（締めの帯が出る）・柵が 4 本・接続が 2 つ**で、中央の列が窓より背高になり、
 * 3 列の入れ物にスクロールが出て、帯のスクロールと 2 本並びました。端末の下端も窓の外へ切れていました。
 */
const ON = 'web-01';
const SIZES = [
	{ width: 1217, height: 757 },
	{ width: 1240, height: 738 },
	{ width: 1400, height: 900 }
] as const;
const ROOTS = ['/srv/app/contents', '/srv/app/libs', '/srv/app/batch', '/srv/app/public/img/notice'];

async function heldByAi(page: Page): Promise<void> {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	const conn = (id: string, name: string, roots: string[]) => ({
		id,
		name,
		host: 'example.invalid',
		port: 22,
		user: 'demo',
		color: null,
		tag: null,
		write_roots: roots
	});
	await fakeTauri(page, {
		connections_list: [
			conn(ON, 'Web server', ROOTS),
			conn('db-01', 'DB server', [])
		],
		session_status: {
			open: [
				{ ...opened(ON, 'Web server'), write: { aiRoots: ROOTS, humanUnrestricted: true } },
				opened('db-01', 'DB server')
			],
			active: ON
		},
		console_holder: { holder: 'ai', connection: ON },
		console_list: [{ id: 5, connection: ON, holder: 'ai' }]
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

/**
 * **字を大きくしたあとも。**`.three` は折り返すので、行の高さはいちばん背の高い列で決まる。
 * 字を大きくした端末が中央の列を押し上げると、入れ物ごとスクロールが出た（中央に上限が無かった）。
 */
const GROW = [
	{ label: '既定の字', steps: 0, from: null },
	{ label: '開いてから字を 2 段大きく', steps: 2, from: null },
	// **字は普通のまま、窓を縮めたとき**（2026-11-10「文字サイズ普通です」）。
	// 最大化から戻す・ノートの画面へ移す、で起きる。端末は縮む前の行数で背高のまま残る。
	{ label: '大きな窓で開いてから縮める', steps: 0, from: { width: 1920, height: 1080 } }
] as const;

for (const size of SIZES)
for (const grow of GROW) {
	test(`端末（AI が握る）· ${size.width}×${size.height} · ${grow.label}: only the columns scroll`, async ({ page }) => {
		await heldByAi(page);
		await page.setViewportSize(grow.from ?? size);
		await page.goto('/');
		await fillBand(page);
		await page.getByRole('button', { name: '端末', exact: true }).click();
		await emitEvent(page, 'console://list', [{ id: 5, connection: ON, holder: 'ai' }]);
		// **端末を出力で埋める**（実機は長い 1 行が折り返して詰まっていた）。
		const long = 'Available versions: ' + Array.from({ length: 80 }, (_, n) => `1.${n}.0`).join(', ');
		for (let n = 0; n < 40; n += 1) {
			await emitEvent(page, 'stream://raw', {
				connection: ON,
				console: 5,
				chunk: `${n % 5 === 0 ? long : `line ${n}`}\r\n`
			});
		}
		await page.waitForTimeout(300);
		for (let n = 0; n < grow.steps; n += 1) await emitEvent(page, 'menu://text-size', 'larger');
		if (grow.from) await page.setViewportSize(size);
		await page.waitForTimeout(300);

		const row = await page.locator('.three').evaluate((el) => {
			const tops = [...el.children].map((c) => Math.round(c.getBoundingClientRect().top));
			return { oneRow: new Set(tops).size === 1, over: el.scrollHeight - el.clientHeight };
		});
		expect(row.oneRow, '試験の前提が崩れた：列が折り返した').toBe(true);
		expect(row.over, '3 列の入れ物にもスクロールが出ている（二重スクロール）').toBeLessThanOrEqual(1);

		// **端末の描きが面の外へはみ出さない**（字を変えたあとも行数が面に合っている）。
		const spill = await page.locator('.pane .terminal').first().evaluate((host) => {
			const screen = host.querySelector('.xterm-screen');
			return screen ? screen.getBoundingClientRect().bottom - host.getBoundingClientRect().bottom : 0;
		});
		expect(spill, '端末の描きが面の下へはみ出している（行数が古い字の大きさのまま）').toBeLessThanOrEqual(1);

		for (const col of ['.three > section.band', '.three .middle']) {
			const box = await page.locator(col).first().boundingBox();
			const bottom = (box?.y ?? 0) + (box?.height ?? 0);
			expect(bottom, `${col} が窓の下で切れている`).toBeLessThanOrEqual(size.height + 1);
		}
	});
}
