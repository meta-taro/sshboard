import { expect, test, type Locator, type Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened } from './fake-tauri';

/**
 * **許可の窓は、読めて押せる大きさにする**（2026-11-26・オーナーの指摘）。
 *
 * > 字が小さくて読めず、ボタンも小さくて押しづらい。
 *
 * 許可の窓は、人が始めていない問いです。**読めない同意は、同意ではありません。**
 * ここだけは他の窓より一回り大きくします（ボタンの高さ 40px 以上・字 15px 以上）。
 */
const MIN_BUTTON_H = 40;
const MIN_BUTTON_FONT = 15;
const MIN_TEXT_FONT = 15;

async function px(locator: Locator, prop: 'height' | 'font-size'): Promise<number> {
	if (prop === 'height') {
		const box = await locator.boundingBox();
		return box?.height ?? 0;
	}
	return locator.evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
}

async function expectReadable(dialog: Locator, textSelector: string) {
	await expect(dialog).toBeVisible();
	const buttons = dialog.locator('.actions button');
	const count = await buttons.count();
	expect(count).toBeGreaterThan(0);
	for (let n = 0; n < count; n++) {
		const button = buttons.nth(n);
		expect(await px(button, 'height'), `button ${n} height`).toBeGreaterThanOrEqual(MIN_BUTTON_H);
		expect(await px(button, 'font-size'), `button ${n} font`).toBeGreaterThanOrEqual(
			MIN_BUTTON_FONT
		);
	}
	const texts = dialog.locator(textSelector);
	const textCount = await texts.count();
	expect(textCount).toBeGreaterThan(0);
	for (let n = 0; n < textCount; n++) {
		expect(await px(texts.nth(n), 'font-size'), `${textSelector} ${n} font`).toBeGreaterThanOrEqual(
			MIN_TEXT_FONT
		);
	}
}

async function open(page: Page) {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened('web-01', 'Web server')], active: 'web-01' }
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
}

test('the operation approval is large enough to read and press', async ({ page }) => {
	await open(page);
	await emitEvent(page, 'operation://request', {
		id: 'reload-nginx',
		runs: 'sudo systemctl reload nginx',
		needsSecret: false,
		connection: 'web-01'
	});
	await expectReadable(page.getByRole('alertdialog'), 'h2, .lead, .note, .runs');
});

test('the console approval is large enough to read and press', async ({ page }) => {
	await open(page);
	await emitEvent(page, 'console://request', 'ai');
	await expectReadable(page.getByRole('alertdialog'), 'h2, .note');
});

test('the host key approval is large enough to read and press', async ({ page }) => {
	await open(page);
	await emitEvent(page, 'host-key://request', {
		id: 'web-01',
		algorithm: 'ssh-ed25519',
		fingerprint: 'SHA256:AAAAexampleexampleexampleexampleexample',
		expected: null
	});
	await expectReadable(page.getByRole('alertdialog'), 'h2, p');
});
