// @vitest-environment jsdom
/**
 * **AI が状態を変える操作を走らせたいときの問い**（D45 / D47 / D48）。
 *
 * ここは**この製品で唯一、人が画面へ秘密を打ち込む所**です（Issue #19）。
 *
 * > `operations.toml` は「**どのコマンドを走らせてよいか**」を解きますが、
 * > 「**どうやって権限を得るか**」は解いていません
 *
 * `become = "ask"` の接続では、**この 1 枚が root へ届く唯一の道**になります。
 * だからこそ、ここで見張るのは 6 つ。
 *
 * 1. **打つものがそのまま出る**（id だけでは答えられない）
 * 2. **要らないときは、欄ごと出さない**（毎回聞かれたら人は使うのをやめる）
 * 3. **要るときは、隠して受ける**（`type="password"`）
 * 4. **焦点は「断る」。**入力欄へ先に飛ばさない —— 読ませてから打たせる
 * 5. **空欄では許可させない**（サーバー側で落ちるだけになる）
 * 6. **答えたら、打ったものを画面から消す**
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import OperationRequestDialog from './OperationRequestDialog.svelte';

beforeEach(() => {
	i18n.set('ja');
});

afterEach(cleanup);

const RUNS = "sudo -S -p '' cat /var/log/maillog";

function mount(needsSecret: boolean) {
	const onAllow = vi.fn();
	const onDeny = vi.fn();
	render(OperationRequestDialog, {
		props: { id: 'read-maillog', runs: RUNS, needsSecret, onAllow, onDeny }
	});
	return { onAllow, onDeny };
}

/** パスワード欄。**役割が付かない入力なので、種類で引きます。** */
function secretField(): HTMLInputElement | null {
	return document.querySelector('input[type="password"]');
}

describe('AI が操作を走らせたいときの問い（D45 / D48）', () => {
	test('shows the command that would actually run, not just its name', () => {
		// **id だけでは答えられません。**書いた本人でも半年後には覚えていません。
		mount(false);

		expect(screen.getByText('read-maillog')).toBeTruthy();
		expect(screen.getByText(RUNS)).toBeTruthy();
	});

	test('asks for no password when the connection does not need one', () => {
		// **要らないのに毎回聞かれたら、人は使うのをやめます。**
		mount(false);

		expect(secretField()).toBeNull();
	});

	test('takes the password hidden when the connection asks for one', () => {
		mount(true);

		const field = secretField();
		expect(field).not.toBeNull();
		// **打ったものが画面に出ない。**肩越しに見られても残らない。
		expect(field?.type).toBe('password');
		// **保存の候補に出さない。**ブラウザ側に覚えさせない。
		expect(field?.autocomplete).toBe('off');
	});

	test('says plainly that what is typed is not kept', () => {
		// **人は「これはどこへ行くのか」を知らずに打てません。**
		mount(true);

		expect(screen.getByText(i18n.t('operation.request.secret.note'))).toBeTruthy();
	});

	test('puts the focus on refusing, even when it is asking for a password', () => {
		// **入力欄へ先に飛ばすと、人は何を許すのかを読む前に打ち始めます。**
		mount(true);

		const focused = document.activeElement as HTMLElement | null;
		expect(focused?.textContent?.trim()).toBe(i18n.t('operation.request.deny'));
	});

	test('does not allow with an empty box', async () => {
		// 空のまま渡すと、サーバー側で「パスワードがありません」に落ちるだけで、
		// **人には何が起きたか分かりません。**
		const { onAllow } = mount(true);

		await fireEvent.click(screen.getByText(i18n.t('operation.request.allow')));

		expect(onAllow).not.toHaveBeenCalled();
		// **代わりに、打つ所へ連れて行く。**黙って何も起きない、を作らない。
		expect(document.activeElement).toBe(secretField());
	});

	test('hands the typed password over exactly once, then clears the box', async () => {
		const { onAllow } = mount(true);
		const field = secretField() as HTMLInputElement;

		await fireEvent.input(field, { target: { value: 'not-a-real-password' } });
		await fireEvent.click(screen.getByText(i18n.t('operation.request.allow')));

		expect(onAllow).toHaveBeenCalledTimes(1);
		expect(onAllow).toHaveBeenCalledWith('not-a-real-password');
		// **渡したあとに残しておく理由がありません。**
		expect(field.value).toBe('');
	});

	test('passes nothing when the connection asks for nothing', async () => {
		// **要らない接続で undefined 以外を渡さない。**空文字を渡すと、
		// engine 側が「人が空欄で許可した」と読み違えます。
		const { onAllow } = mount(false);

		await fireEvent.click(screen.getByText(i18n.t('operation.request.allow')));

		expect(onAllow).toHaveBeenCalledWith(undefined);
	});

	test('refuses on Escape and keeps nothing that was typed', async () => {
		const { onAllow, onDeny } = mount(true);
		const field = secretField() as HTMLInputElement;
		await fireEvent.input(field, { target: { value: 'not-a-real-password' } });

		await fireEvent.keyDown(window, { key: 'Escape' });

		expect(onDeny).toHaveBeenCalledTimes(1);
		expect(onAllow).not.toHaveBeenCalled();
		expect(field.value).toBe('');
	});
});
