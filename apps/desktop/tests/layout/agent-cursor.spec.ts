import { expect, test, type Page } from '@playwright/test';

import { calledWith, emitEvent } from './fake-tauri';
import { stage } from './scenes';

/**
 * **エージェント専用カーソル**（2026-11-12・オーナーの依頼・tadori の仕様が正本）。
 *
 * 本体（MCP の `screen`）から `agent://act` が届くと、画面は専用のカーソルを動かし、
 * 結果を `agent_act_done` で返す。**人の OS のマウスは動かさない。**
 *
 * sshboard では押せる物を絞っています（`agent_screen.rs` に理由）。押せるのは
 * `data-agent-view` の印が付いたタブだけです。
 *
 * 「画面が開いていなければ理由を返す」は画面の無い所の話なので、
 * `crates/sshboard-mcp/tests/agent_screen.rs` の `without_a_screen_it_says_to_open_one_first` が見ます。
 */

let nextId = 1;

function idOf(args: unknown): number | undefined {
	return (args as { id?: number } | null)?.id;
}

/** 指示を 1 つ送り、画面の答えを待って返す。 */
async function act(page: Page, payload: Record<string, unknown>): Promise<string> {
	const id = nextId++;
	await emitEvent(page, 'agent://act', { id, act: payload });
	await expect
		.poll(async () => (await calledWith(page, 'agent_act_done')).some((c) => idOf(c.args) === id))
		.toBe(true);
	const done = (await calledWith(page, 'agent_act_done')).find((c) => idOf(c.args) === id);
	return (done?.args as { outcome: string }).outcome;
}

/** 人のマウスが動いたら数える。**エージェントは 1 回も動かさないはず。** */
async function countHumanMouse(page: Page) {
	await page.evaluate(() => {
		(window as unknown as { __mouse: number }).__mouse = 0;
		document.addEventListener('mousemove', () => {
			(window as unknown as { __mouse: number }).__mouse += 1;
		});
	});
}

test.beforeEach(async ({ page }) => {
	await stage(page, 'fresh', { locale: 'en', theme: 'light', width: 1100, height: 720 });
});

test('point shows the cursor and a frame on the thing it names', async ({ page }) => {
	// Act
	const outcome = await act(page, { action: 'point', target: 'Files', message: 'Look here' });

	// Assert
	expect(outcome).toBe('done');
	await expect(page.locator('.agent-cursor')).toBeVisible();
	await expect(page.locator('.agent-box')).toBeVisible();
	await expect(page.locator('.agent-bubble')).toContainText('Look here');
	const tab = await page.locator('nav.tabs [data-agent-view="files"]').boundingBox();
	const box = await page.locator('.agent-box').boundingBox();
	expect(tab && box).toBeTruthy();
	// 枠は指した物を囲む（4px 外側）。
	expect(Math.abs((box?.x ?? 0) - ((tab?.x ?? 0) - 4))).toBeLessThanOrEqual(2);
});

test('click presses a tab marked for agents without moving the human mouse', async ({ page }) => {
	// Arrange
	await countHumanMouse(page);
	const band = page.locator('nav.tabs [data-agent-view="band"]');
	await expect(band).not.toHaveClass(/active/);
	const label = (await band.textContent())?.trim() ?? '';

	// Act
	const outcome = await act(page, { action: 'click', target: label });

	// Assert
	expect(outcome).toBe('done');
	await expect(band).toHaveClass(/active/);
	expect(await page.evaluate(() => (window as unknown as { __mouse: number }).__mouse)).toBe(0);
});

test('a button not marked for agents is pointed at but never pressed', async ({ page }) => {
	// Arrange —— 人が押すべき物の代わり（印の無い釦）。
	await page.evaluate(() => {
		const b = document.createElement('button');
		b.textContent = 'Allow this run';
		b.style.cssText = 'position:fixed;left:300px;top:300px;z-index:50';
		b.addEventListener('click', () => {
			(window as unknown as { __pressed: boolean }).__pressed = true;
		});
		document.body.append(b);
	});

	// Act
	const outcome = await act(page, { action: 'click', target: 'Allow this run' });

	// Assert
	expect(outcome).toBe('not_allowed');
	expect(
		await page.evaluate(() => (window as unknown as { __pressed?: boolean }).__pressed)
	).toBeFalsy();
});

test('something not on screen is reported and nothing is pressed', async ({ page }) => {
	// Arrange
	const before = await page.locator('nav.tabs .active').textContent();

	// Act
	const outcome = await act(page, { action: 'click', target: 'No such thing anywhere' });

	// Assert
	expect(outcome).toBe('not_found');
	await expect(page.locator('nav.tabs .active')).toHaveText(before ?? '');
});

test('the bubble stays inside the window near every corner', async ({ page }) => {
	// Arrange —— 四隅に指す相手を置く。
	await page.evaluate(() => {
		const spots: [string, string][] = [
			['Corner TL', 'left:0;top:0'],
			['Corner TR', 'right:0;top:0'],
			['Corner BL', 'left:0;bottom:0'],
			['Corner BR', 'right:0;bottom:0']
		];
		for (const [name, where] of spots) {
			const b = document.createElement('button');
			b.textContent = name;
			b.style.cssText = `position:fixed;${where};z-index:50`;
			document.body.append(b);
		}
	});
	const view = page.viewportSize() ?? { width: 0, height: 0 };

	for (const name of ['Corner TL', 'Corner TR', 'Corner BL', 'Corner BR']) {
		// Act
		const outcome = await act(page, {
			action: 'point',
			target: name,
			message: 'A message long enough to wrap onto a second line inside the bubble.'
		});

		// Assert —— 窓の端から 8px 内側。
		expect(outcome).toBe('done');
		const bubble = await page.locator('.agent-bubble').boundingBox();
		expect(bubble, name).toBeTruthy();
		expect(bubble!.x, name).toBeGreaterThanOrEqual(7);
		expect(bubble!.y, name).toBeGreaterThanOrEqual(7);
		expect(bubble!.x + bubble!.width, name).toBeLessThanOrEqual(view.width - 7);
		expect(bubble!.y + bubble!.height, name).toBeLessThanOrEqual(view.height - 7);
	}
});
