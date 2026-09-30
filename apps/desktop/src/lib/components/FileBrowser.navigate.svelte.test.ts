// @vitest-environment jsdom
/**
 * **［↑］が「一つ上」へ戻る**（Issue #8・2026-09-30 に実機で再現）。
 *
 * 実機の言葉 ——
 *
 * > **上押したのに元の表示じゃないですね**
 * > **ルートに移動してます**
 *
 * リモート側は `.`（入った所）から始まります。`app` へ入って［↑］を押すと、
 * **`/`（実機の根）の一覧が出ていました。**まったく別の場所です。
 *
 * ## なぜ画面ごと試すか
 *
 * `parentOf` 単体の試験だけでは、**この形を捕まえられませんでした。**
 * 入る側が `remotePath === '.' ? name : joinPath(...)` と特別扱いしていて、
 * **`.` から入ると `app`（`/` を含まない相対パス）になる**——
 * その組み合わせで初めて根へ落ちます。**部品を 2 つ繋いだ所に穴がありました。**
 *
 * なので、**押して、次に何を読みに行ったか**を見ます。
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
// **窓へのドラッグ＆ドロップの受け口**。jsdom には窓が無いので、差し替える。
vi.mock('@tauri-apps/api/webview', () => ({
	getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} })
}));

import { i18n } from '$lib/i18n/i18n.svelte';
import { session } from '$lib/session.svelte';

import FileBrowser from './FileBrowser.svelte';

/** リモートの中身。**場所ごとに別のものを返す**（どこを読みに行ったかが分かる）。 */
const REMOTE: Record<string, { name: string; isDir: boolean; size: number }[]> = {
	'.': [{ name: 'app', isDir: true, size: 0 }],
	app: [{ name: 'logs', isDir: true, size: 0 }],
	'app/logs': [{ name: 'today.log', isDir: false, size: 12 }],
	'/': [{ name: 'etc', isDir: true, size: 0 }]
};

/** `remote_list_dir` に渡された場所を、呼ばれた順に控える。 */
let asked: string[] = [];

beforeEach(() => {
	asked = [];
	invoke.mockReset();
	invoke.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
		if (cmd === 'connections_list') return Promise.resolve([]);
		if (cmd === 'session_status') {
			return Promise.resolve({ open: session.all, active: session.activeId });
		}
		if (cmd === 'local_list_dir') {
			return Promise.resolve({ path: '/home/me', parent: '/home', entries: [] });
		}
		if (cmd === 'remote_list_dir') {
			const path = String(args?.path);
			asked.push(path);
			return Promise.resolve(REMOTE[path] ?? []);
		}
		return Promise.resolve(undefined);
	});
	session.all = [
		{
			id: 'test',
			name: '手元のテスト用サーバー',
			fingerprint: 'SHA256:xxxx',
			hostKeyAlgorithm: 'ssh-ed25519',
			write: { aiRoots: [], humanUnrestricted: true }
		}
	];
	session.activeId = 'test';
});

afterEach(() => {
	cleanup();
	session.all = [];
	session.activeId = null;
});

/** リモート側の［↑］。**手元側にも同じ札のボタンが在る**ので、後ろの方を取る。 */
function remoteUp(): HTMLElement {
	const all = screen.getAllByLabelText(i18n.t('files.up'));
	return all[all.length - 1];
}

describe('リモート側の移動', () => {
	test('goes back to where it started, not to the filesystem root', async () => {
		render(FileBrowser);
		await waitFor(() => expect(asked).toContain('.'));

		// app へ入る
		await fireEvent.click(await screen.findByRole('button', { name: 'app' }));
		await waitFor(() => expect(asked).toContain('app'));

		// ［↑］を押す
		await fireEvent.click(remoteUp());

		// **根を読みに行っていないこと。**入った所へ戻ること。
		await waitFor(() => expect(asked[asked.length - 1]).toBe('.'));
		expect(asked).not.toContain('/');
	});

	test('climbs one level at a time from a deeper place', async () => {
		render(FileBrowser);
		await waitFor(() => expect(asked).toContain('.'));

		await fireEvent.click(await screen.findByRole('button', { name: 'app' }));
		await waitFor(() => expect(asked).toContain('app'));
		await fireEvent.click(await screen.findByRole('button', { name: 'logs' }));
		await waitFor(() => expect(asked).toContain('app/logs'));

		await fireEvent.click(remoteUp());
		await waitFor(() => expect(asked[asked.length - 1]).toBe('app'));

		await fireEvent.click(remoteUp());
		await waitFor(() => expect(asked[asked.length - 1]).toBe('.'));
		expect(asked).not.toContain('/');
	});
});
