// @vitest-environment jsdom
/**
 * **画面から接続を切れる**（2026-11-12・オーナーの指示）。
 *
 * > 画面に［切断］ボタンがありません
 *
 * 釦はありましたが、**絵（プラグ）だけで字が無く**、名前は読み上げと乗せたときの説明にしか
 * ありませんでした。人から見て「切断」という機能が見つからない作りでした。
 * そのうえ繋ぎ直そうとすると「先に切ってから繋ぎ直してください」と断られ、行き止まりでした。
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
vi.mock('@tauri-apps/api/webview', () => ({
	getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} })
}));

import { i18n } from '$lib/i18n/i18n.svelte';
import { session } from '$lib/session.svelte';

import FileBrowser from './FileBrowser.svelte';

beforeEach(() => {
	invoke.mockReset();
	invoke.mockImplementation((cmd: string) => {
		if (cmd === 'connections_list') return Promise.resolve([]);
		if (cmd === 'session_status')
			return Promise.resolve({ open: session.all, active: session.activeId });
		if (cmd === 'local_list_dir') {
			return Promise.resolve({ path: '/home/me', parent: '/home', entries: [] });
		}
		if (cmd === 'remote_list_dir') return Promise.resolve([]);
		return Promise.resolve(undefined);
	});
	session.all = [
		{
			id: 'web-01',
			name: 'Web server',
			fingerprint: 'SHA256:xxxx',
			hostKeyAlgorithm: 'ssh-ed25519',
			write: { aiRoots: [], humanUnrestricted: true }
		}
	];
	session.activeId = 'web-01';
});

afterEach(() => {
	cleanup();
	session.all = [];
	session.activeId = null;
});

test('the disconnect button says so in visible text, not only in a tooltip', async () => {
	// Arrange
	render(FileBrowser);

	// Act
	const button = await screen.findByRole('button', { name: i18n.t('files.disconnect') });

	// Assert —— 字が画面に出ている（読み上げ名・説明だけではない）。
	expect(button.textContent?.trim()).toContain(i18n.t('files.disconnect'));
});

test('pressing it closes that connection', async () => {
	// Arrange
	render(FileBrowser);
	const button = await screen.findByRole('button', { name: i18n.t('files.disconnect') });

	// Act
	await fireEvent.click(button);

	// Assert
	await waitFor(() => expect(invoke).toHaveBeenCalledWith('session_disconnect', { id: 'web-01' }));
});
