import { expect, test, type Page } from '@playwright/test';

import { fakeTauri, opened } from './fake-tauri';

/**
 * **端末を開いたら、そのまま打てる**（実機の指摘・2026-11-16）。
 *
 * > 端末を開くを押したあと、端末内にフォーカスがないのがかなりストレスです。
 * > 繋いだ後はやることが一つだからです
 *
 * 開いたあとに、もう 1 回面を押さないと打てませんでした。
 * **焦点が端末の中（xterm の入力欄）に在ること**を測ります。
 */
const VIEWING = 'web-01';

async function setup(page: Page, holder: unknown): Promise<void> {
	await page.addInitScript(() => {
		try {
			localStorage.setItem('sshboard-locale', 'ja');
		} catch {
			// 保存できなくても既定の言語で測れる。
		}
	});
	await fakeTauri(page, {
		session_status: { open: [opened(VIEWING, 'Web server')], active: VIEWING },
		console_holder: holder,
		console_list: [],
		connections_list: [],
		console_open_another: 2
	});
	await page.setViewportSize({ width: 1100, height: 700 });
	await page.goto('/');
}

/**
 * 開いたあとに実行体が返す一覧。**開く前は 0 枚**なので、押す前に差し替える。
 *
 * 握り手（`console_holder`）は差し替えない —— 「誰も握っていない」が自動で開く条件なので、
 * 先に「人」にすると開かずに終わります。開いた握りは画面が自分で「人」にします。
 */
async function afterOpening(page: Page, id: number): Promise<void> {
	await page.evaluate(
		([consoleId, connection]) => {
			const replies = (window as unknown as { __fakeReplies: Record<string, unknown> })
				.__fakeReplies;
			replies.console_list = [{ id: consoleId, connection, holder: 'human' }];
		},
		[id, VIEWING] as const
	);
}

/** 焦点が、端末の面の xterm の中に在るか。 */
function typingInTerminal(page: Page): Promise<boolean> {
	return page.evaluate(
		() => document.activeElement?.closest('.console-panes .xterm') != null
	);
}

test('pressing open on an empty pane leaves the keyboard in the new terminal', async ({ page }) => {
	// 別の接続で握っているので、自動では開かない（空の面の釦を押す道）。
	await setup(page, { holder: 'human', connection: 'db-01' });
	await page.getByRole('button', { name: '端末', exact: true }).click();
	await afterOpening(page, 2);

	await page.locator('.console-panes').getByRole('button', { name: '端末を開く' }).click();

	await expect.poll(() => typingInTerminal(page)).toBe(true);
});

test('the console that opens by itself after connecting can be typed into at once', async ({
	page
}) => {
	// 誰も握っていなければ、端末の面へ入ると自動で開く（`openConsole` の道）。
	await setup(page, null);
	await afterOpening(page, 1);

	await page.getByRole('button', { name: '端末', exact: true }).click();

	await expect.poll(() => typingInTerminal(page)).toBe(true);
});
