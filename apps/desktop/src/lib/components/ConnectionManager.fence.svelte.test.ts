// @vitest-environment jsdom
/**
 * **柵が、接続を選ぶ画面にも出ていること**（UI 案・2026-10-21／D22）。
 *
 * 柵（AI が書いてよいディレクトリ）は**接続ごとに人が決めるもの**なのに、
 * **接続の一覧には出ていませんでした** ——
 * どの接続で AI が書けるのかを、一覧では比べられなかった。
 *
 * ## 空を警告に見せない
 *
 * **既定は空＝AI は 1 バイトも書けない**（D22）。それが普通の状態であり、
 * 設定し忘れではありません。**空を色で主張すると一覧が全部光り、
 * 本当に柵が在る接続が埋もれます。**
 *
 * ## 全部並べない
 *
 * 柵が何本も在る接続は珍しくありません（案の例も 2 本）。
 * **全部並べると行が伸びて、一覧そのものが読めなくなります。**
 * 件数と最初の 1 本だけ出します。
 */
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));

import { i18n } from '$lib/i18n/i18n.svelte';

import ConnectionManager from './ConnectionManager.svelte';

/**
 * 登録された接続 1 件（合成値）。
 *
 * **名前と識別子を別にします。**行は両方を出すので、同じにすると
 * `getByText` が 2 つ見つけて落ちます（最初そうして落ちました）。
 */
function entry(id: string, name: string, roots: string[] | undefined) {
	return {
		id,
		name,
		host: 'example.test',
		port: 22,
		user: 'ops',
		color: 'teal',
		...(roots === undefined ? {} : { write_roots: roots })
	};
}

const LIST = [
	entry('web-01', 'Fenced', ['/srv/app/releases', '/var/tmp/batch']),
	entry('db-01', 'Bare', []),
	// **古い `connections.toml` から来た場合。**`write_roots` が無い。
	entry('old-01', 'Legacy', undefined)
];

beforeEach(() => {
	invoke.mockReset();
	invoke.mockImplementation((cmd: string) => {
		if (cmd === 'connections_list') return Promise.resolve(LIST);
		if (cmd === 'connections_path') return Promise.resolve('/tmp/connections.toml');
		return Promise.resolve(undefined);
	});
});

afterEach(() => cleanup());

describe('柵を接続の一覧に出す', () => {
	test('says how many places the AI may write in', async () => {
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Fenced')).toBeTruthy());

		expect(screen.getByText(i18n.t('conn.fence.some', { count: '2' }))).toBeTruthy();
	});

	test('shows the first place, so the count is not the only clue', async () => {
		// 「2 か所」だけでは、**どこなのか**が分かりません。
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Fenced')).toBeTruthy());

		expect(screen.getByText('/srv/app/releases')).toBeTruthy();
	});

	test('does not list every place in the row', async () => {
		// **行が伸びると一覧が読めません。**2 本目は出しません。
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Fenced')).toBeTruthy());

		expect(screen.queryByText('/var/tmp/batch')).toBeNull();
	});

	test('says plainly that an empty fence means read only', async () => {
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Bare')).toBeTruthy());

		// 空の接続が 2 件あるので、両方に出ます。
		expect(screen.getAllByText(i18n.t('conn.fence.none')).length).toBeGreaterThanOrEqual(2);
	});

	test('treats a missing field as no fence, not as unknown', async () => {
		// **古い `connections.toml` から来たとき**は `write_roots` が無い。
		// **書けない側に倒す** —— 分からないものを「書ける」と見せない。
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Legacy')).toBeTruthy());

		const row = screen.getByText('Legacy').closest('button');
		expect(row?.textContent).toContain(i18n.t('conn.fence.none'));
	});

	test('does not colour an empty fence as a warning', async () => {
		// **既定は空。**そこを光らせると一覧が全部光り、本物が埋もれます。
		render(ConnectionManager);
		await waitFor(() => expect(screen.getByText('Bare')).toBeTruthy());

		const bare = screen.getByText('Bare').closest('button');
		const mark = bare?.querySelector('.row-fence');
		expect(mark, '柵の行が見つかりません').toBeTruthy();
		expect(mark?.classList.contains('open')).toBe(false);

		const fenced = screen.getByText('Fenced').closest('button');
		expect(fenced?.querySelector('.row-fence')?.classList.contains('open')).toBe(true);
	});
});
