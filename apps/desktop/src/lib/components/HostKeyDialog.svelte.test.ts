// @vitest-environment jsdom
/**
 * **ホスト鍵を承認する所**（Issue #26）を、実際に描いて確かめる。
 *
 * 実機の報告（0.1.13 / Windows）——
 *
 * > 赤帯に `[object Object]` とだけ表示され、［承認］［拒否］が描画されない
 * > **アプリは人の答えを待ち続けるが、人には答える手段が無い**
 *
 * **#24 で型だけ作って、画面へ繋いでいませんでした。**
 * **見えるのに答えられないのは、見えないより悪い**です ——
 * 人は「反応しない」と読み、同じ操作を繰り返します。
 *
 * 見張るのは 5 つ。
 *
 * 1. **指紋と algorithm が読める形で出る**（`[object Object]` にしない）
 * 2. **［承認］と［拒否］が両方在る**
 * 3. **既定の焦点は［拒否］**（開いた勢いの Enter で通さない）
 * 4. **登録と食い違うときは、すり替えの疑いとして別の見せ方**
 * 5. **接続先を出さない。**出すのは識別子だけ（PRD §8）
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import HostKeyDialog from './HostKeyDialog.svelte';

const FINGERPRINT = 'SHA256:AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJKKK';
const OTHER = 'SHA256:ZZZZYYYYXXXXWWWWVVVVUUUUTTTTSSSSRRRRQQQQPPP';

beforeEach(() => {
	i18n.set('ja');
});

afterEach(cleanup);

function mount(expected: string | null = null) {
	const onTrust = vi.fn();
	const onRefuse = vi.fn();
	render(HostKeyDialog, {
		props: {
			id: 'web-prod',
			algorithm: 'ssh-ed25519',
			fingerprint: FINGERPRINT,
			expected,
			busy: false,
			onTrust,
			onRefuse
		}
	});
	return { onTrust, onRefuse };
}

describe('ホスト鍵の問い', () => {
	test('shows the fingerprint and the algorithm as readable text', () => {
		// **`[object Object]` にしない。**人はこれを見て承認するので、
		// **読めないものを出すのは、答えを求めていないのと同じ**です。
		mount();

		expect(screen.getByText(FINGERPRINT)).toBeTruthy();
		expect(screen.getByText(/ssh-ed25519/)).toBeTruthy();
	});

	test('offers both an approve and a refuse button', () => {
		const { onTrust, onRefuse } = mount();

		fireEvent.click(screen.getByRole('button', { name: i18n.t('hostkey.trust') }));
		expect(onTrust).toHaveBeenCalledTimes(1);

		fireEvent.click(screen.getByRole('button', { name: i18n.t('hostkey.refuse') }));
		expect(onRefuse).toHaveBeenCalledTimes(1);
	});

	test('puts the focus on refuse, not on approve', () => {
		// **開いた勢いで Enter を打っても、承認になりません。**
		// 端末の許可（D42）と同じ考え方です。
		mount();

		expect(document.activeElement?.textContent?.trim()).toBe(i18n.t('hostkey.refuse'));
	});

	test('says it plainly when the key does not match what is registered', () => {
		// **初見と、食い違いは別物です。**
		// 食い違いは**すり替えの疑い**で、承認は「登録を書き換える」ことになります。
		mount(OTHER);

		expect(screen.getByText(new RegExp(i18n.t('hostkey.mismatch')))).toBeTruthy();
		expect(screen.getByText(OTHER)).toBeTruthy();
	});

	test('never shows the host or the user', () => {
		// **接続先を画面へ出さない**（PRD §8 / CLAUDE.md 禁止事項 4）。
		// 出るのは識別子だけ。
		mount();

		const text = document.body.textContent ?? '';
		expect(text).toContain('web-prod');
		expect(text).not.toMatch(/@|\bport\b|192\.168\./i);
	});
});
