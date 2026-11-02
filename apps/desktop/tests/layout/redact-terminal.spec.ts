import { expect, test, type Page } from '@playwright/test';

import { emitEvent } from './fake-tauri';
import { stage } from './scenes';

/**
 * **伏せたら、端末の中の字も伏せる**（2026-11-02・カムスタの実機・capture_window）。
 *
 * 伏せて撮ったのに、**端末の中に出ている前回のログイン元の IP と、プロンプトのホスト名**が
 * そのまま写った。伏せる印（data-secret）は自分の字の色だけを消すので、
 * xterm が字ごとに色を付けて描く端末の中には届いていなかった。
 * **この製品は画面そのものに接続先が写り込む**（禁止事項 4）。
 */
const SECRET = 'Last login: from 192.0.2.77\r\n[demo@host-secret-01 ~]$ ';

async function writeToConsole(page: Page) {
	// 画面は開いている端末を `console://list` で知る。知らせないと端末の面が立たない。
	await emitEvent(page, 'console://list', [{ id: 1, connection: 'c0', holder: 'human' }]);
	await page.waitForTimeout(300);
	const bytes = Array.from(new TextEncoder().encode(SECRET));
	await emitEvent(page, 'stream://raw', { connection: 'c0', console: 1, chunk: bytes });
	await emitEvent(page, 'stream://raw', { connection: 'c0', console: null, chunk: bytes });
}

/** 字の見える要素を集める（色が透明でないもの）。 */
async function readableText(page: Page, scope: string): Promise<string[]> {
	return page.evaluate((selector) => {
		const out: string[] = [];
		for (const root of document.querySelectorAll(selector)) {
			const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
			for (let node = walker.nextNode(); node; node = walker.nextNode()) {
				const text = node.textContent?.trim();
				const el = node.parentElement;
				if (!text || !el) continue;
				// 部品が差し込む書式（<style>）は画面の字ではない。
				if (el.closest('style, script')) continue;
				const color = getComputedStyle(el).color;
				const transparent = /rgba\(.*,\s*0\)$/.test(color) || color === 'transparent';
				if (!transparent) out.push(text);
			}
		}
		return out;
	}, scope);
}

for (const [label, tab, scope] of [
	['端末の面', 2, '.console-panes'],
	['出力の面', 3, '.stream']
] as const) {
	test(`hides what the shell printed in ${label} once hiding is on`, async ({ page }) => {
		await stage(page, 'busy', { locale: 'ja', theme: 'dark', width: 1100, height: 700 });
		await page.locator('nav.tabs button').nth(tab).click();
		await page.waitForTimeout(300);
		await writeToConsole(page);
		await page.waitForTimeout(300);
		// 伏せる前は読める（試験の前提）。
		const before = (await readableText(page, scope)).join(' ');
		expect(before, '端末に字が出ていない（試験の前提が崩れた）').toContain('host-secret-01');

		await page.locator('button.mask-toggle').click();
		await page.waitForTimeout(200);
		const after = (await readableText(page, scope)).join(' ');
		expect(after, `**伏せたのに読める**: ${after}`).not.toContain('host-secret-01');
		expect(after).not.toContain('192.0.2.77');
	});
}
