// @vitest-environment jsdom
/**
 * **選んだものが、場所を移ったあとも残っていないこと。**
 *
 * サーバー側で落とすものを選ぶとき、画面は**名前だけ**を持ちます
 * （`pickedRemote`）。絶対パスにしないのは、いま見えている階層のものしか
 * 落とさないためです。
 *
 * **残ると何が起きるか** —— `remote_download` には
 * `{ names, remoteDir }` が渡ります。名前が残ったまま `remoteDir` が
 * 変わると、**選んだ覚えのないファイルが落ちます。**
 *
 * 一番悪いのは**接続を切り替えたとき**です。`logs/today.log` を選んで
 * 別の台のタブを押すと、`remotePath` は `.` に戻ります —— 名前が残っていれば
 * **別の台の同じ名前**を落とします。画面には「落としました」と出ます。
 * 人は、どちらの台のものが手元に在るのか分かりません。
 *
 * ## 上書きの許しも 1 回きり（product-baseline §13）
 *
 * > 破壊的な操作は、実行前に人へ確認する。
 * > **一度承認された操作が、次も承認されているとは限らない。**
 *
 * 落としたあとにチェックが入ったままだと、**次の 1 回は黙って上書きします。**
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
vi.mock('@tauri-apps/api/webview', () => ({
	getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} })
}));

import { i18n } from '$lib/i18n/i18n.svelte';
import { session } from '$lib/session.svelte';

import FileBrowser from './FileBrowser.svelte';

/** リモートの中身。**どちらの台にも `today.log` が在る**のが要点。 */
const REMOTE: Record<string, { name: string; isDir: boolean; size: number }[]> = {
	'.': [
		{ name: 'logs', isDir: true, size: 0 },
		{ name: 'today.log', isDir: false, size: 99 }
	],
	logs: [{ name: 'today.log', isDir: false, size: 12 }]
};

/** `remote_download` に渡された引数。**落とす直前の画面の持ち物がここに出る。** */
let downloads: Array<{ names: string[]; remoteDir: string; overwrite: boolean }> = [];

function connection(id: string, name: string) {
	return {
		id,
		name,
		fingerprint: 'SHA256:xxxx',
		hostKeyAlgorithm: 'ssh-ed25519',
		write: { aiRoots: [], humanUnrestricted: true }
	};
}

beforeEach(() => {
	downloads = [];
	invoke.mockReset();
	invoke.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
		if (cmd === 'connections_list') return Promise.resolve([]);
		if (cmd === 'session_status') {
			return Promise.resolve({ open: session.all, active: session.activeId });
		}
		if (cmd === 'local_list_dir') {
			return Promise.resolve({ path: '/home/me', parent: '/home', entries: [] });
		}
		if (cmd === 'remote_list_dir') return Promise.resolve(REMOTE[String(args?.path)] ?? []);
		if (cmd === 'remote_download') {
			downloads.push({
				names: args?.names as string[],
				remoteDir: String(args?.remoteDir),
				overwrite: Boolean(args?.overwrite)
			});
			return Promise.resolve([{ name: 'today.log', bytes: 12 }]);
		}
		return Promise.resolve(undefined);
	});
	session.all = [connection('a', '1 台目'), connection('b', '2 台目')];
	session.activeId = 'a';
});

afterEach(() => {
	cleanup();
	session.all = [];
	session.activeId = null;
});

/** サーバー側の［↑］。**手元側にも同じ札のボタンが在る**ので、後ろの方を取る。 */
function remoteUp(): HTMLElement {
	const all = screen.getAllByLabelText(i18n.t('files.up'));
	return all[all.length - 1];
}

/**
 * 「落とす」のボタン。
 *
 * **札に件数が入る**（`Download {count}`）ので、件数が 0 のときと 1 のときで
 * 文字が変わります。**0 件のとき（＝捨てられたこと）を確かめたい**のだから、
 * 件数を含む文字で探すと、そもそも見つけられません。
 * なので**件数を抜いた、言語ごとの決まり文句の方**で探します。
 */
const DOWNLOAD_LABEL = i18n
	.t('files.download', { count: '\u0000' })
	.split('\u0000')
	.map((part) => part.trim())
	.reduce((longest, part) => (part.length > longest.length ? part : longest), '');

function downloadButton(): HTMLButtonElement {
	const found = screen
		.getAllByRole('button')
		.find((b) => b.textContent?.includes(DOWNLOAD_LABEL));
	if (!found) throw new Error(`落とすボタンが見つかりません（${DOWNLOAD_LABEL} で探した）`);
	return found as HTMLButtonElement;
}

/** `logs/today.log` を選んだ状態まで進める。 */
async function pickInsideLogs() {
	render(FileBrowser);
	await fireEvent.click(await screen.findByRole('button', { name: 'logs' }));
	await fireEvent.click(await screen.findByText('today.log'));
	await waitFor(() => expect(downloadButton().disabled).toBe(false));
}

describe('選んだものは、場所を移ったら捨てる', () => {
	test('clears the picked file when climbing out of the directory', async () => {
		await pickInsideLogs();

		await fireEvent.click(remoteUp());

		// **一つ上にも `today.log` が在る。**選択が残っていれば押せてしまう。
		await waitFor(() => expect(downloadButton().disabled).toBe(true));
		expect(downloads).toEqual([]);
	});

	test('clears the picked file when switching to another connection', async () => {
		await pickInsideLogs();

		// 別の台のタブを押す（= AI が宛先を動かしたときも同じ経路）
		session.activeId = 'b';

		// **「繋がっていないから押せない」で通ってしまわないように**、
		// まず 2 台目の一覧が出ていることを確かめる。`.` には `today.log` が
		// 在るので、選択が残っていればここで押せてしまう。
		await waitFor(() => expect(screen.getByRole('button', { name: 'logs' })).toBeTruthy());
		expect(screen.getByText('today.log')).toBeTruthy();

		expect(downloadButton().disabled).toBe(true);
		expect(downloads).toEqual([]);
	});

	test('sends the directory it was picked in, not the one shown later', async () => {
		await pickInsideLogs();
		await fireEvent.click(downloadButton());

		await waitFor(() => expect(downloads).toHaveLength(1));
		expect(downloads[0]).toMatchObject({ names: ['today.log'], remoteDir: 'logs' });
	});
});

describe('上書きの許しは 1 回きり（§13）', () => {
	test('unchecks overwrite after the download finishes', async () => {
		await pickInsideLogs();

		const box = screen.getByLabelText(i18n.t('files.overwrite')) as HTMLInputElement;
		await fireEvent.click(box);
		expect(box.checked).toBe(true);

		await fireEvent.click(downloadButton());
		await waitFor(() => expect(downloads).toHaveLength(1));
		expect(downloads[0].overwrite).toBe(true);

		// **次の 1 回に持ち越さない。**持ち越すと、黙って上書きする。
		// 落としたあとは選択が空なのでチェックの欄も消える。選び直して出す。
		await fireEvent.click(await screen.findByText('today.log'));
		await waitFor(() => expect(downloadButton().disabled).toBe(false));
		expect((screen.getByLabelText(i18n.t('files.overwrite')) as HTMLInputElement).checked).toBe(
			false
		);
	});
});
