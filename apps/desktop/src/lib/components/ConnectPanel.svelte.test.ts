// @vitest-environment jsdom
/**
 * **失敗の帯が、いつ消えるか**を確かめる（Issue #26 のコメント・2026-09-24）。
 *
 * 実機からの報告 ——
 *
 * > 赤帯の `[object Object]` が**前の失敗の残り**として画面上部に出続けており、
 * > それをパスフレーズの問いだと取り違えました。
 * > **帯は消えずに残り続ける**ようで、これが誤認の原因になりました。
 *
 * **古い帯が残ると、人は「いま起きていること」として読みます。**
 * その人は、これを見て別の不具合として起票し、あとで自分で訂正しました。
 * **1 本の誤った報告と、2 往復のやり取りが、この帯から生まれています。**
 *
 * 帯を消していたのは 2 か所だけでした ——
 * `connect()` の頭と、パスフレーズの問いを［やめる］したとき。
 * **別の接続を選び直しても残ります。**選び直した時点で、
 * その帯が指している相手は、もう画面に無いのに。
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { i18n } from '$lib/i18n/i18n.svelte';
import { session } from '$lib/session.svelte';

import ConnectPanel from './ConnectPanel.svelte';

const REGISTERED = [
	{ id: 'web-prod', name: 'Web', color: 'teal', tag: null, fingerprint: null },
	{ id: 'db-prod', name: 'DB', color: 'indigo', tag: null, fingerprint: null }
];

beforeEach(() => {
	session.busy = false;
	invoke.mockReset();
	// **`i18n.set` より先に置く。**あれは `set_menu_labels` を呼び、
	// 返りに `.catch` を繋ぐので、**実装を入れる前に呼ぶと undefined を掴みます。**
	invoke.mockImplementation((cmd: string) => {
		if (cmd === 'connections_list') return Promise.resolve(REGISTERED);
		if (cmd === 'session_connect') {
			return Promise.reject({ kind: 'other', message: '鍵が合いません' });
		}
		return Promise.resolve(undefined);
	});
	i18n.set('ja');
});

afterEach(cleanup);

/** 帯を 1 本出してから返す。 */
async function raiseBand() {
	render(ConnectPanel, { props: {} });
	await screen.findByText('Web');
	await fireEvent.click(screen.getByText('Web'));
	await fireEvent.click(screen.getByRole('button', { name: i18n.t('files.connect') }));
	// **jest-dom の matcher はこのリポに入っていません。**素の textContent で見ます。
	await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('鍵が合いません'));
}

describe('失敗の帯', () => {
	test('繋ぎ損ねたら、理由が帯に出る', async () => {
		await raiseBand();
	});

	test('**別の接続を選び直したら、前の失敗の帯は消える**', async () => {
		await raiseBand();

		// 別の相手を選ぶ。**帯が指していた相手は、もう選ばれていない。**
		await fireEvent.click(screen.getByText('DB'));

		await waitFor(() =>
			expect(screen.queryByRole('alert')).toBeNull()
		);
	});
});
