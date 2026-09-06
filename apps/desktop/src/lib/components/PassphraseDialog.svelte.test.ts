// @vitest-environment jsdom
/**
 * **鍵のパスフレーズを聞く所**（Issue #7 / D11 / D14）を、実際に描いて確かめる。
 *
 * ここは**秘密が通る唯一の画面**です。方針は「**繋ぐたびに人が入れる。保存しない**」
 * で、Issue #7 で変えたのは**聞き方だけ**でした。
 *
 * 見張るのは、その方針が守られていること。
 *
 * 1. **渡したら、その場で画面から捨てる**（D14）
 * 2. **打っている間、中身が見えない**（肩越しに読まれない）
 * 3. **接続先を出さない。**出すのは識別子だけ（PRD §8）
 * 4. **空では送らない。**空のまま鍵へ渡すと、読めない理由が分かりにくくなる
 * 5. **繋ぎに行っている最中は、消せない**（途中で消えると宙に浮く）
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import PassphraseDialog from './PassphraseDialog.svelte';

/** **合成値。**実物の秘密はテストにも置きません（product-baseline §14）。 */
const SECRET = 'ひみつの合言葉';

beforeEach(() => {
	i18n.set('ja');
});

afterEach(cleanup);

function mount(busy = false) {
	const onSubmit = vi.fn();
	const onCancel = vi.fn();
	render(PassphraseDialog, { props: { id: 'web-prod', busy, onSubmit, onCancel } });
	const field = screen.getByLabelText(i18n.t('files.passphrase')) as HTMLInputElement;
	return { onSubmit, onCancel, field };
}

/**
 * 人が打つ。
 *
 * **素の `dispatchEvent` では `bind:value` に届きませんでした**（実測）。
 * testing-library の口を通します。
 */
async function type(field: HTMLInputElement, value: string) {
	await fireEvent.input(field, { target: { value } });
}

describe('パスフレーズを聞く所（D14）', () => {
	test('never shows what is being typed', () => {
		// **肩越しに読まれない。**画面を人に見せながら繋ぐ場面があります。
		const { field } = mount();

		expect(field.type).toBe('password');
		expect(field.autocomplete).toBe('off');
	});

	test('shows only the connection id, never a host or a user', () => {
		// **接続先を成果物へ出さない**（PRD §8 / CLAUDE.md 禁止事項 4）。
		mount();

		const shown = document.body.textContent ?? '';
		expect(shown).toContain('web-prod');
		for (const leak of ['@', '://', '192.168.', '10.0.', '.com', '.local']) {
			expect(shown.includes(leak), `問いに接続先らしきものが出ている（${leak}）`).toBe(false);
		}
	});

	test('hands the passphrase over and drops it from the screen at once', async () => {
		// **渡した先が使い終わるのを待たない**（D14）。
		const { onSubmit, field } = mount();
		await type(field, SECRET);

		const connect = screen.getByRole('button', {
			name: i18n.t('files.connect')
		}) as HTMLButtonElement;
		// **打てば押せるようになること。**押せないままだと、次の assert が
		// 「呼ばれていない」で落ち、原因が入力側だと分かりません。
		expect(connect.disabled).toBe(false);
		connect.click();

		expect(onSubmit).toHaveBeenCalledWith(SECRET);
		// **画面に残らないこと。**残ると、次に開いた人がそのまま送れます。
		await vi.waitFor(() => expect(field.value).toBe(''));
		expect(document.body.innerHTML.includes(SECRET)).toBe(false);
	});

	test('refuses to send an empty passphrase', () => {
		// 空のまま鍵へ渡すと、**読めない理由が分かりにくくなります。**
		const { onSubmit } = mount();

		const connect = screen.getByRole('button', {
			name: i18n.t('files.connect')
		}) as HTMLButtonElement;

		expect(connect.disabled).toBe(true);
		connect.click();
		expect(onSubmit).not.toHaveBeenCalled();
	});

	test('lets Escape cancel while it is waiting for the person', () => {
		const { onCancel } = mount();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

		expect(onCancel).toHaveBeenCalledTimes(1);
	});

	test('does not let Escape cancel while it is connecting', () => {
		// **途中で消えると宙に浮きます。**繋ぎに行っている最中は答えを変えさせない。
		const { onCancel } = mount(true);

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

		expect(onCancel).not.toHaveBeenCalled();
	});
});
