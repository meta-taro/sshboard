// @vitest-environment jsdom
/**
 * **許可を続けている操作の一覧**（D47 改・2026-10-26）。
 *
 * > その恒久許可をあとから確認・解除できる設定の枠がほしい。
 *
 * 見張るのは 4 つ。
 * 1. **どの操作を・どの接続で**許しているかが並ぶ
 * 2. 1 件ずつ外せる（外すと実行体へ伝わる）
 * 3. 何も無いときは、無いと言う
 * 4. **接続の名前は伏せられる**（禁止事項 4）
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import StandingDialog from './StandingDialog.svelte';

beforeEach(() => i18n.set('ja'));
afterEach(cleanup);

const LIST = [
	{ operation: 'pm2-list', connection: 'web-01' },
	{ operation: 'pm2-restart', connection: 'db-01' }
];

describe('許可を続けている操作の一覧', () => {
	test('lists what is allowed and where', () => {
		render(StandingDialog, { props: { allowances: LIST, onRevoke: vi.fn(), onClose: vi.fn() } });
		expect(screen.getByText('pm2-list')).toBeTruthy();
		expect(screen.getByText('db-01')).toBeTruthy();
	});

	test('takes one away when its button is pressed', async () => {
		const onRevoke = vi.fn();
		render(StandingDialog, { props: { allowances: LIST, onRevoke, onClose: vi.fn() } });
		const buttons = screen.getAllByRole('button', { name: i18n.t('standing.revoke') });
		await fireEvent.click(buttons[1]);
		expect(onRevoke).toHaveBeenCalledWith(LIST[1]);
	});

	test('says so when nothing is allowed', () => {
		render(StandingDialog, { props: { allowances: [], onRevoke: vi.fn(), onClose: vi.fn() } });
		expect(screen.getByText(i18n.t('standing.empty'))).toBeTruthy();
	});

	test('lets the connection names be hidden', () => {
		render(StandingDialog, { props: { allowances: LIST, onRevoke: vi.fn(), onClose: vi.fn() } });
		expect(screen.getByText('web-01').hasAttribute('data-secret')).toBe(true);
	});
});
