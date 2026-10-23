// @vitest-environment jsdom
/**
 * **繋がっている接続の並びと、AI の柵。**
 *
 * ## ここで見るいちばん大事なこと
 *
 * **柵は「AI が書ける所」を示す印**です（D22）。
 * ずれると**嘘の安心**になるので、次の 3 つを数えます。
 *
 * 1. **空のときは色で主張しない。**既定は空（AI は読むだけ）で、
 *    それが普通の状態。警告色にすると**画面が警告だらけになり、本物が埋もれます。**
 * 2. **「人は制限されない」を必ず添える。**書かないと、印を見た人が
 *    「自分も書けないのか」と読みます。
 * 3. **道を全部出す。**端末の横に常設する列なので、ここは件数では足りません
 *    （接続の一覧の方は行が伸びるので最初の 1 本だけ）。
 *
 * ## 横でも縦でも同じ中身
 *
 * 案（UI 案・2026-10-21）は縦の列、実物はいま横並び。
 * **写して 2 つ置かない**ため 1 つの部品にしたので、
 * **どちらの置き方でも中身が変わらないこと**も見ます。
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, test } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import ConnectionRail, { type RailItem } from './ConnectionRail.svelte';

const ITEMS: RailItem[] = [
	{ id: 'web-01', name: 'Web', tag: 'prod' },
	{ id: 'db-01', name: 'Database', tag: null }
];

/** 既定の入口。 */
function mount(over: Record<string, unknown> = {}) {
	return render(ConnectionRail, {
		items: ITEMS,
		activeId: 'web-01',
		onpick: () => {},
		onadd: () => {},
		...over
	});
}

afterEach(() => cleanup());

describe('接続の並び', () => {
	test('shows every connection it was given', () => {
		mount();
		expect(screen.getByRole('tab', { name: /Web/ })).toBeTruthy();
		expect(screen.getByRole('tab', { name: /Database/ })).toBeTruthy();
	});

	test('says which one is chosen, not only by colour', () => {
		mount();
		expect(screen.getByRole('tab', { name: /Web/ }).getAttribute('aria-selected')).toBe('true');
		expect(screen.getByRole('tab', { name: /Database/ }).getAttribute('aria-selected')).toBe(
			'false'
		);
	});

	test('hands back the identifier when one is pressed', () => {
		const picked: string[] = [];
		mount({ onpick: (id: string) => picked.push(id) });

		fireEvent.click(screen.getByRole('tab', { name: /Database/ }));
		expect(picked).toEqual(['db-01']);
	});

	test('lets the connection name be hidden', () => {
		// **画面そのものに接続先が写り込みます**（D26 / 禁止事項 4）。
		mount();
		expect(screen.getByText('Web').hasAttribute('data-secret')).toBe(true);
		expect(screen.getByText('prod').hasAttribute('data-secret')).toBe(true);
	});

	test('offers opening another connection, in words', () => {
		// **アイコンだけだと、触れる前の人には存在しません**（Issue #9）。
		let added = 0;
		mount({ onadd: () => (added += 1) });

		fireEvent.click(screen.getByRole('button', { name: new RegExp(i18n.t('files.another')) }));
		expect(added).toBe(1);
	});
});

describe('AI の柵（D22）', () => {
	test('says plainly that an empty fence means read only', () => {
		mount({ roots: [] });
		expect(screen.getByText(i18n.t('files.scope.none'))).toBeTruthy();
	});

	test('does not colour an empty fence as a warning', () => {
		// **既定は空。**そこを光らせると画面が警告だらけになり、本物が埋もれます。
		const { container } = mount({ roots: [] });
		expect(container.querySelector('.fence')?.classList.contains('open')).toBe(false);
	});

	test('colours it as the AI once there is a fence', () => {
		const { container } = mount({ roots: ['/srv/app/releases'] });
		expect(container.querySelector('.fence')?.classList.contains('open')).toBe(true);
	});

	test('shows every place the AI may write', () => {
		// **端末の横に常設する列なので、件数では足りません。**
		mount({ roots: ['/srv/app/releases', '/var/tmp/batch'] });
		expect(screen.getByText('/srv/app/releases')).toBeTruthy();
		expect(screen.getByText('/var/tmp/batch')).toBeTruthy();
	});

	test('lets those places be hidden too', () => {
		// **道はサーバー側のパス。**接続先そのものです（禁止事項 4）。
		mount({ roots: ['/srv/app/releases'] });
		expect(screen.getByText('/srv/app/releases').hasAttribute('data-secret')).toBe(true);
	});

	test('tells the person they are not the one being fenced', () => {
		// **人は制限されません**（D22）。書かないと自分が縛られていると読まれます。
		mount({ roots: ['/srv/app/releases'] });
		expect(screen.getByText(i18n.t('files.scope.help'))).toBeTruthy();
	});

	test('does not leave a dangling colon in the heading', () => {
		// 見出しは `files.scope.some` から道を抜いて作ります。
		// **抜き方が崩れると「AI が書けるのは:」のような尻切れ**が出ます。
		const { container } = mount({ roots: ['/srv/app/releases'] });
		const head = container.querySelector('.fence-head');
		expect(head, '柵の見出しが見つかりません').toBeTruthy();
		expect(head?.textContent?.trim()).not.toMatch(/[:：]$/u);
		expect((head?.textContent?.trim().length ?? 0) > 2).toBe(true);
	});
});

describe('横でも縦でも', () => {
	test('keeps the same contents in the vertical layout', () => {
		mount({ layout: 'rail', roots: ['/srv/app/releases'] });

		expect(screen.getByRole('tab', { name: /Web/ })).toBeTruthy();
		expect(screen.getByText('/srv/app/releases')).toBeTruthy();
		expect(screen.getByText(i18n.t('files.scope.help'))).toBeTruthy();
	});

	test('marks which way it is laid out', () => {
		const { container } = mount({ layout: 'rail' });
		expect(container.querySelector('.rail')?.classList.contains('vertical')).toBe(true);
	});

	test('stays horizontal by default', () => {
		// **既定はいまの形。**部品に出しただけで見た目が変わらないこと。
		const { container } = mount();
		expect(container.querySelector('.rail')?.classList.contains('vertical')).toBe(false);
	});
});
