// @vitest-environment jsdom
/**
 * **入力エラーが、エラーに見える**（Issue #32・2026-10-01）。
 *
 * オーナーの言葉 ——
 *
 * > 接続を作ろうとしたところ、「識別子は英数字と . _ - だけにしてください」と
 * > 表示されました。**文言は出ていますが、エラーだと一目で分かりません。**
 *
 * 直接の原因は、止めている理由を `--fg-faint`（**薄い灰色**）で描いていたことです。
 * 依頼は「**赤字で／問題のある入力欄に赤い枠線か薄い赤の背景**」。
 *
 * **色だけに頼りません。**`aria-invalid` と印も付けます
 * （色の見分けが難しい人に、色は届きません）。
 *
 * ## 打ち始める前から赤くしない
 *
 * 新規の登録は**全部空から始まります。**空を即座に赤くすると、
 * **何も間違えていないのに画面が赤だらけ**になり、本当の間違いが埋もれます。
 * **一度その欄を離れてから**赤くします。
 */
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));

import { i18n } from '$lib/i18n/i18n.svelte';

import ConnectionManager from './ConnectionManager.svelte';

beforeEach(() => {
	invoke.mockReset();
	invoke.mockImplementation((cmd: string) => {
		if (cmd === 'connections_list') return Promise.resolve([]);
		if (cmd === 'connections_path') return Promise.resolve('/tmp/connections.toml');
		return Promise.resolve(undefined);
	});
});

afterEach(() => cleanup());

/** 識別子の欄。**label の文字で引く。** */
async function idField(): Promise<HTMLInputElement> {
	return (await screen.findByPlaceholderText('web-prod')) as HTMLInputElement;
}

describe('止めている理由の見え方', () => {
	test('does not paint the field before it has been left', async () => {
		render(ConnectionManager);
		const id = await idField();

		// 打っただけ。**まだ離れていない。**
		await fireEvent.input(id, { target: { value: 'web prod' } });

		expect(id.getAttribute('aria-invalid')).not.toBe('true');
		expect(id.className).not.toContain('invalid');
	});

	test('marks the field once it has been left', async () => {
		render(ConnectionManager);
		const id = await idField();

		await fireEvent.input(id, { target: { value: 'web prod' } });
		await fireEvent.blur(id);

		await waitFor(() => expect(id.getAttribute('aria-invalid')).toBe('true'));
		expect(id.className).toContain('invalid');
	});

	test('says it out loud, not only in colour', async () => {
		render(ConnectionManager);
		const id = await idField();
		await fireEvent.input(id, { target: { value: 'web prod' } });
		await fireEvent.blur(id);

		// **読み上げにも届く**こと。色の見分けが難しい人のため。
		const said = await screen.findByRole('alert');
		expect(said.textContent).toContain(i18n.t('conn.err.id.chars'));
	});

	test('clears the mark once the value is fixed', async () => {
		render(ConnectionManager);
		const id = await idField();
		await fireEvent.input(id, { target: { value: 'web prod' } });
		await fireEvent.blur(id);
		await waitFor(() => expect(id.getAttribute('aria-invalid')).toBe('true'));

		await fireEvent.input(id, { target: { value: 'web-prod' } });

		// **直した瞬間に消える。**保存を押すまで待たせない。
		await waitFor(() => expect(id.getAttribute('aria-invalid')).not.toBe('true'));
	});
});
