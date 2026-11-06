import { expect, test, type Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened } from './fake-tauri';
import { lookFindings } from './look';

/**
 * **点検の 2 枚にも、ほかの面と同じ物差しを当てる**（D71・2026-11-06）。
 *
 * `look.spec.ts` は 5 つの面を 3 つの大きさ × 日英 × 明暗で検めています。
 * 点検の画面と案の画面は、入れた日に 1100×700・日本語でしか測っていませんでした。
 * **中身は長めに**（長い値・長い理由・長い説明）—— 崩れは短い見本では出ません。
 */
const SIZES = [
	{ width: 860, height: 600 },
	{ width: 1100, height: 700 },
	{ width: 1400, height: 900 }
] as const;
const LONG = 'Filesystem 1K-blocks Used Available Use% Mounted on /dev/mapper/vg-root 98%';

const REPORT = {
	id: 'morning-check-for-the-production-web-servers',
	description: 'Every morning before nine: disk, memory, the web server, and the TLS listener.',
	connection: 'web-01',
	steps: Array.from({ length: 6 }, (_, n) => ({
		readonly: `step-${n}`,
		title: n === 0 ? 'Root filesystem usage in percent (should stay well below ninety)' : `Step ${n}`,
		judged: {
			verdict:
				n % 3 === 0
					? { kind: 'good' }
					: n % 3 === 1
						? { kind: 'bad', why: `「active」が出ていません — ${LONG}` }
						: { kind: 'unknown', why: 'sshboard の許可リストに在りません（在るもの: disk , uptime , nginx）' },
			value: n === 0 ? LONG : `${40 + n}%`
		},
		previous: { verdict: { kind: 'good' }, value: `${39 + n}%` }
	})),
	good: 2,
	bad: 2,
	unknown: 2
};

const PROPOSAL = {
	id: REPORT.id,
	description: REPORT.description,
	step: [
		{ readonly: 'disk-root', label: 'Root filesystem usage in percent (should stay well below ninety)', below: 90 },
		{ readonly: 'nginx-active', contains: 'active', absent: 'failed' },
		{ readonly: 'mem-available-mb', above: 200 },
		{ readonly: 'uptime' }
	]
};

async function stage(page: Page, locale: 'ja' | 'en', size: { width: number; height: number }) {
	await page.addInitScript((wanted) => {
		try {
			localStorage.setItem('sshboard-locale', wanted);
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	}, locale);
	await fakeTauri(page, {
		session_status: { open: [opened('web-01', 'Web server')], active: 'web-01' },
		checks_overview: {
			path: '/home/someone/.config/sshboard/checks.toml',
			checks: [
				{ id: REPORT.id, description: REPORT.description, steps: [], last: REPORT },
				{ id: 'weekly', description: null, steps: [], last: null }
			]
		},
		check_proposal: null
	});
	await page.setViewportSize(size);
	await page.goto('/');
}

for (const locale of ['ja', 'en'] as const) {
	for (const scheme of ['light', 'dark'] as const) {
		for (const size of SIZES) {
			const at = `${locale} · ${scheme} · ${size.width}×${size.height}`;

			test(`checks dialog · ${at}`, async ({ page }) => {
				await page.emulateMedia({ colorScheme: scheme });
				await stage(page, locale, size);
				await page.locator('button.brand').click();
				await page.locator('.view-menu button', { hasText: locale === 'ja' ? '点検' : 'Health checks' }).click();
				await expect(page.getByRole('dialog').locator('tbody tr')).toHaveCount(6);

				expect(await lookFindings(page)).toEqual([]);
			});

			test(`check proposal · ${at}`, async ({ page }) => {
				await page.emulateMedia({ colorScheme: scheme });
				await stage(page, locale, size);
				await emitEvent(page, 'check://proposal', PROPOSAL);
				await expect(page.getByRole('dialog').locator('tbody tr')).toHaveCount(4);

				expect(await lookFindings(page)).toEqual([]);
			});
		}
	}
}
