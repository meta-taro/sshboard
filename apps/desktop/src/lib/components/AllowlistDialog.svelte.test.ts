// @vitest-environment jsdom
/**
 * **AI が走らせられることの一覧**（2026-10-23）。
 *
 * ## なぜ要るか
 *
 * **人だけが見られませんでした。**
 *
 * 実行体は `readonly.toml` と `operations.toml` を読めて、AI も
 * `list_readonly_commands` / `list_operations` で読めるのに、
 * **画面へ出す口が 1 つもありませんでした。**
 *
 * 本番（2026-10-23）で詰まったのはここです ——
 * 人は書く場所を探して見つけられず、**操作は sshboard の外で行われました。**
 * 置き場を書いても、**書いた物が入ったかを確かめる所が無ければ同じです。**
 *
 * ## ここで見るいちばん大事なこと
 *
 * **「空」と「読めない」を混ぜないこと。**
 * 書き間違いを「0 本」と出すと、人は**書いたのに断られる**所で止まります。
 */
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, test } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import AllowlistDialog, { type AllowedList } from './AllowlistDialog.svelte';

const READING: AllowedList = {
	path: '/somewhere/readonly.toml',
	lines: [{ id: 'disk', run: 'df -h', why: '空きを見る', maxPerHour: null }],
	trouble: null
};

const CHANGING: AllowedList = {
	path: '/somewhere/operations.toml',
	lines: [
		{
			id: 'httpd-configtest',
			run: 'sudo apachectl configtest',
			why: '書式を確かめる',
			maxPerHour: 20
		}
	],
	trouble: null
};

const EMPTY: AllowedList = { path: '/somewhere/operations.toml', lines: [], trouble: null };

function show(changing: AllowedList = CHANGING, reading: AllowedList = READING) {
	render(AllowlistDialog, { reading, changing, onclose: () => {} });
}

afterEach(() => cleanup());

describe('AI が走らせられること', () => {
	test('shows what is written in both lists', () => {
		show();
		expect(screen.getByText('df -h')).toBeTruthy();
		expect(screen.getByText('sudo apachectl configtest')).toBeTruthy();
	});

	test('says where the file is, because that is what was not found', () => {
		// **本番で詰まったのはここ。**人は書く場所を探して見つけられませんでした。
		show();
		expect(screen.getByText('/somewhere/operations.toml')).toBeTruthy();
		expect(screen.getByText('/somewhere/readonly.toml')).toBeTruthy();
	});

	test('says plainly that an empty list runs nothing', () => {
		// **「0 本」では足りません。**「だから 1 本も走らない」まで書く。
		show(EMPTY);
		expect(screen.getByText(i18n.t('allow.empty'))).toBeTruthy();
	});

	test('does not show a broken file as an empty one', () => {
		// **ここが一番大事。**書き間違いを 0 本と出すと、
		// 人は「書いたのに断られる」所で止まります。
		const broken: AllowedList = {
			path: '/somewhere/operations.toml',
			lines: [],
			trouble: 'operations.toml を読めません: id が重なっています'
		};
		show(broken);
		expect(screen.getByText(broken.trouble as string)).toBeTruthy();
		expect(screen.queryByText(i18n.t('allow.empty'))).toBeNull();
	});

	test('shows the ceiling, so a runaway is visibly bounded', () => {
		show();
		expect(screen.getByText(i18n.t('allow.ceiling', { count: '20' }))).toBeTruthy();
	});

	test('lets the commands be hidden', () => {
		// **コマンドにはサーバー側の道とドメインが入ります**（禁止事項 4）——
		// `-d example.com` や `/var/www/html`。画面共有の前に伏せられること。
		show();
		for (const run of ['df -h', 'sudo apachectl configtest']) {
			expect(screen.getByText(run).hasAttribute('data-secret'), `${run} に印が無い`).toBe(true);
		}
	});

	test('hides the file location too', () => {
		// 置き場には手元の利用者名が入ります。
		show();
		expect(screen.getByText('/somewhere/operations.toml').hasAttribute('data-secret')).toBe(true);
	});

	test('can be closed', () => {
		let closed = false;
		render(AllowlistDialog, {
			reading: READING,
			changing: CHANGING,
			onclose: () => (closed = true)
		});
		screen.getByRole('button', { name: i18n.t('about.close') }).click();
		expect(closed).toBe(true);
	});
});
