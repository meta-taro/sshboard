import { expect, test, type Page } from '@playwright/test';

import { calledWith, emitEvent } from './fake-tauri';
import { stage } from './scenes';

/**
 * **AI が「この接続のこのフォルダを開いて」と頼む**（`show_view files`・2026-11-13・PdM #2035）。
 *
 * 人に設定フォルダへファイルを置いてもらう場面で、AI にできたのはタブの切り替えまでで、
 * 人が上から手でたどりました。**頼まれた接続・フォルダで開き、ファイルを選んだ状態にする。**
 *
 * フォルダが在るか・読めるかは本体が先に確かめます（`crates/sshboard-mcp/tests/show_view_opens_a_folder.rs`）。
 * ここは画面の側だけを見ます。
 */

const FOLDER = '/srv/app/conf';
const FILE = 'remote-file-7.log';

async function ask(page: Page, id: number): Promise<string> {
	await emitEvent(page, 'agent://act', {
		id,
		act: { action: 'open', connection_id: 'c4', path: FOLDER, select: FILE }
	});
	const answered = async () =>
		(await calledWith(page, 'agent_act_done')).find(
			(c) => (c.args as { id?: number } | null)?.id === id
		);
	await expect.poll(async () => Boolean(await answered())).toBe(true);
	return ((await answered())?.args as { outcome: string }).outcome;
}

test.beforeEach(async ({ page }) => {
	await stage(page, 'busy', { locale: 'en', theme: 'light', width: 1100, height: 720 });
});

test('opens the files tab on the named connection and folder, with the file selected', async ({
	page
}) => {
	// Act
	const outcome = await ask(page, 1);

	// Assert
	expect(outcome).toBe('done');
	await expect(page.locator('nav.tabs [data-agent-view="files"]')).toHaveClass(/active/);
	await expect
		.poll(async () => (await calledWith(page, 'session_focus')).map((c) => c.args))
		.toContainEqual({ id: 'c4' });
	await expect(page.locator(`[data-remote-entry="${FILE}"]`)).toHaveClass(/picked/);
});

test('the folder it was asked for is not reset to the top by the connection switch', async ({
	page
}) => {
	// Act
	await ask(page, 2);

	// Assert —— 最後に読んだのは頼まれたフォルダ（宛先が変わった所で `.` へ戻さない）。
	await expect(page.locator(`[data-remote-entry="${FILE}"]`)).toBeVisible();
	const reads = (await calledWith(page, 'remote_list_dir')).map((c) => c.args);
	expect(reads.at(-1)).toEqual({ path: FOLDER });
});

test('does not move the screen while the person is typing', async ({ page }) => {
	// Arrange —— 人が欄に打っている。
	await page.evaluate(() => {
		const field = document.createElement('input');
		document.body.append(field);
		field.focus();
	});
	const before = await page.locator('nav.tabs .active').getAttribute('data-agent-view');

	// Act
	const outcome = await ask(page, 3);

	// Assert
	expect(outcome).toBe('busy');
	await expect(page.locator('nav.tabs .active')).toHaveAttribute(
		'data-agent-view',
		before ?? ''
	);
});
