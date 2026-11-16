// @vitest-environment jsdom
/**
 * **端末の期限付きの許可**（2026-11-16）。
 *
 * 測定用の機械（時間で課金）へ AI が繋ぐたび、人が［許可］を押すまで止まっていました。
 * **人が先に「いまから N 時間は聞かずに通す」を出せる**枠です。
 *
 * 見張るのは 5 つ。
 * 1. **既定は「これから増える接続だけ」**（人がふだん使っている接続には効かない側）
 * 2. 期限は 1・3・24 時間から選ぶ（上限は 24 時間）
 * 3. 開いている接続があれば「この接続だけ」も選べる。無ければ出さない
 * 4. 出ている許可は、範囲と残りの時間と一緒に出て、外せる
 * 5. **接続の名前は伏せられる**（禁止事項 4）
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import ConsoleGrantDialog from './ConsoleGrantDialog.svelte';

beforeEach(() => i18n.set('ja'));
afterEach(cleanup);

const CURRENT = { id: 'vm-01', name: 'measure vm' };

function props(over: Record<string, unknown> = {}) {
	return {
		current: CURRENT,
		grant: null,
		onGrant: vi.fn(),
		onRevoke: vi.fn(),
		onClose: vi.fn(),
		...over
	};
}

describe('端末の期限付きの許可', () => {
	test('grants new connections only for 3 hours by default', async () => {
		const onGrant = vi.fn();
		render(ConsoleGrantDialog, { props: props({ onGrant }) });
		await fireEvent.click(screen.getByRole('button', { name: i18n.t('grant.give') }));
		expect(onGrant).toHaveBeenCalledWith('new', null, 3);
	});

	test('can grant only the open connection, for the chosen hours', async () => {
		const onGrant = vi.fn();
		render(ConsoleGrantDialog, { props: props({ onGrant }) });
		await fireEvent.click(screen.getByRole('radio', { name: /measure vm/ }));
		await fireEvent.click(screen.getByRole('radio', { name: i18n.t('grant.hours', { hours: 24 }) }));
		await fireEvent.click(screen.getByRole('button', { name: i18n.t('grant.give') }));
		expect(onGrant).toHaveBeenCalledWith('connection', 'vm-01', 24);
	});

	test('offers no hours above the 24-hour limit', () => {
		render(ConsoleGrantDialog, { props: props() });
		const hours = screen
			.getAllByRole('radio')
			.map((radio) => radio.getAttribute('value'))
			.filter((value) => value !== null && /^\d+$/.test(value))
			.map(Number);
		expect(hours).toEqual([1, 3, 24]);
	});

	test('does not offer "this connection" when nothing is open', () => {
		render(ConsoleGrantDialog, { props: props({ current: null }) });
		expect(screen.queryByRole('radio', { name: /measure vm/ })).toBeNull();
		expect(screen.getByRole('radio', { name: i18n.t('grant.scope.new') })).toBeTruthy();
	});

	test('shows the grant in force with the time left, and takes it away', async () => {
		const onRevoke = vi.fn();
		render(ConsoleGrantDialog, {
			props: props({
				onRevoke,
				grant: { scope: 'new', connection: null, remainingSecs: 125 * 60 }
			})
		});
		expect(screen.getByText(i18n.t('grant.left', { minutes: 125 }))).toBeTruthy();
		await fireEvent.click(screen.getByRole('button', { name: i18n.t('grant.revoke') }));
		expect(onRevoke).toHaveBeenCalled();
	});

	test('says so when nothing is granted', () => {
		render(ConsoleGrantDialog, { props: props() });
		expect(screen.getByText(i18n.t('grant.none'))).toBeTruthy();
	});

	test('lets the connection name be hidden', () => {
		render(ConsoleGrantDialog, { props: props() });
		expect(screen.getByText('measure vm').hasAttribute('data-secret')).toBe(true);
	});
});
