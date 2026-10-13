// @vitest-environment jsdom
/**
 * **上げ待ちは、落とし待ちとわざと逆の規則で持っています。**
 *
 * | | 持ち方 | 階層を移ったら |
 * |---|---|---|
 * | 落とす（サーバー → 手元） | **名前だけ** | **捨てる** |
 * | 上げる（手元 → サーバー） | **絶対パス** | **残す** |
 *
 * 揃っていないのは手抜きではありません。
 *
 * - **落とす側**は `{ names, remoteDir }` を渡すので、名前を残したまま
 *   `remoteDir` が変わると**選んだ覚えのないファイルが落ちます。**
 * - **上げる側**は `{ localPaths }` に絶対パスを渡すので、
 *   どこを見ていようと**選んだそのファイル**が上がります。
 *   むしろ捨てると、**あちこちから集めて一度に上げる**ことができません。
 *
 * **「片側に揃える」と、どちらかが壊れます。**落とす側を絶対パスにすると
 * 階層を移っても選択が残り、上げる側を名前だけにすると集められなくなる。
 * だから両方を、理由ごと見張ります。
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
vi.mock('@tauri-apps/api/webview', () => ({
	getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} })
}));
// **［選ぶ］が開く OS のダイアログ。**jsdom には無いので、同じものを返す形に差し替える。
const pickResult = { value: [] as string[] | string | null };
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: async () => pickResult.value }));

import { i18n } from '$lib/i18n/i18n.svelte';
import { session } from '$lib/session.svelte';

import FileBrowser from './FileBrowser.svelte';

/** 手元の中身。**両方の階層に `notes.txt` が在る**のが要点。 */
const LOCAL: Record<string, { path: string; parent: string | null; entries: unknown[] }> = {
	'/home/me': {
		path: '/home/me',
		parent: '/home',
		entries: [
			{ name: 'work', isDir: true, size: 0 },
			{ name: 'notes.txt', isDir: false, size: 7 }
		]
	},
	'/home/me/work': {
		path: '/home/me/work',
		parent: '/home/me',
		entries: [{ name: 'notes.txt', isDir: false, size: 42 }]
	}
};

let uploads: Array<{ localPaths: string[]; remoteDir: string }> = [];

beforeEach(() => {
	uploads = [];
	pickResult.value = [];
	invoke.mockReset();
	invoke.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
		if (cmd === 'connections_list') return Promise.resolve([]);
		if (cmd === 'session_status') {
			return Promise.resolve({ open: session.all, active: session.activeId });
		}
		if (cmd === 'local_list_dir') {
			const asked = args?.path == null ? '/home/me' : String(args.path);
			return Promise.resolve(LOCAL[asked] ?? { path: asked, parent: null, entries: [] });
		}
		if (cmd === 'remote_list_dir') return Promise.resolve([]);
		if (cmd === 'remote_upload') {
			uploads.push({
				localPaths: args?.localPaths as string[],
				remoteDir: String(args?.remoteDir)
			});
			return Promise.resolve([{ name: 'notes.txt', bytes: 42 }]);
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

/**
 * 「上げる」のボタン。**札に件数が入る**（`Upload {count}`）ので、
 * 0 件のときを確かめたいこちらは、件数を抜いた決まり文句で探します。
 */
const UPLOAD_LABEL = i18n
	.t('files.upload', { count: '\u0000' })
	.split('\u0000')
	.map((part) => part.trim())
	.reduce((longest, part) => (part.length > longest.length ? part : longest), '');

function uploadButton(): HTMLButtonElement {
	const found = screen.getAllByRole('button').find((b) => b.textContent?.includes(UPLOAD_LABEL));
	if (!found) throw new Error(`上げるボタンが見つかりません（${UPLOAD_LABEL} で探した）`);
	return found as HTMLButtonElement;
}

/** 手元側の一覧に出ている名前を押す（手元の面が先に描かれる）。 */
async function clickLocal(name: string) {
	const all = await screen.findAllByRole('button', { name });
	await fireEvent.click(all[0]);
}

describe('上げ待ちは、階層を移っても残す', () => {
	test('keeps the staged file after moving into another directory', async () => {
		render(FileBrowser);
		await clickLocal('notes.txt'); // /home/me/notes.txt を選ぶ
		await waitFor(() => expect(uploadButton().disabled).toBe(false));

		await clickLocal('work'); // 別の階層へ入る
		await waitFor(() => expect(screen.getByText('42 B')).toBeTruthy());

		// **捨てない。**捨てると、あちこちから集めて一度に上げられない。
		expect(uploadButton().disabled).toBe(false);
	});

	test('collects files from more than one directory', async () => {
		render(FileBrowser);
		await clickLocal('notes.txt');
		await waitFor(() => expect(uploadButton().disabled).toBe(false));

		await clickLocal('work');
		await waitFor(() => expect(screen.getByText('42 B')).toBeTruthy());
		await clickLocal('notes.txt'); // 同じ名前・別の場所

		await fireEvent.click(uploadButton());
		await waitFor(() => expect(uploads).toHaveLength(1));

		// **同じ名前でも別のものとして 2 つ上がる**（絶対パスで持っているから）。
		expect(uploads[0].localPaths).toEqual(['/home/me/notes.txt', '/home/me/work/notes.txt']);
	});

	/**
	 * **［選ぶ］を二度押しても増えないこと。**
	 *
	 * > 同じものを二度並べない。**押すたびに増えるのは事故のもと。**
	 *
	 * 一覧を押す方（`toggle`）は選ぶ／外すなので、作りの上で重複しません。
	 * **重なるのはダイアログの方**です —— 同じファイルを選んで［選ぶ］を
	 * もう一度押すと、同じ絶対パスが 2 つ並び、**同じものを 2 回上げます。**
	 */
	test('does not stage the same file twice when picked again from the dialog', async () => {
		render(FileBrowser);
		pickResult.value = ['/home/me/notes.txt'];

		const pick = await screen.findByRole('button', { name: i18n.t('files.pick') });
		await fireEvent.click(pick);
		await waitFor(() => expect(uploadButton().disabled).toBe(false));
		await fireEvent.click(pick); // 同じものをもう一度

		await fireEvent.click(uploadButton());
		await waitFor(() => expect(uploads).toHaveLength(1));
		expect(uploads[0].localPaths).toEqual(['/home/me/notes.txt']);
	});
});

describe('上げたら、上げ待ちを空にする', () => {
	test('clears the staged list so a second press does not resend', async () => {
		render(FileBrowser);
		await clickLocal('notes.txt');
		await waitFor(() => expect(uploadButton().disabled).toBe(false));

		await fireEvent.click(uploadButton());
		await waitFor(() => expect(uploads).toHaveLength(1));

		// **残すと、もう一度押したときに黙って同じものを上げます。**
		await waitFor(() => expect(uploadButton().disabled).toBe(true));
		expect(uploads).toHaveLength(1);
	});
});
