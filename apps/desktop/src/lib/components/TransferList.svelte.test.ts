// @vitest-environment jsdom
/**
 * **転送の列**（UI 案・2 枚目）。**誰が動かしたかが行ごとに残ること。**
 *
 * ## ここで見るいちばん大事なこと
 *
 * **止めたことが見えること。**
 *
 * それまで「柵の外へ書こうとして断られた」は、帯の 1 行に埋もれていました。
 * **止まったのが見えないと、人は「動いた」と思って次へ進みます** ——
 * 本番（2026-10-23）で起きたのは、まさにそういう取り違えの連なりでした。
 *
 * そして**止めた行に大きさを出しません** ——
 * 「4.0 KB」と並んでいたら、人は動いたと読みます。
 */
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, test } from 'vitest';

import { i18n } from '$lib/i18n/i18n.svelte';

import TransferList, { type TransferLine } from './TransferList.svelte';

const LINES: TransferLine[] = [
	{
		seq: 1,
		actor: 'human',
		direction: 'up',
		what: '/srv/app/releases/build.tar.gz',
		bytes: 12_400_000,
		outcome: 'done'
	},
	{
		seq: 2,
		actor: 'ai',
		direction: 'down',
		what: '/var/log/app/error.log',
		bytes: 4096,
		outcome: 'done'
	},
	{
		seq: 3,
		actor: 'ai',
		direction: 'up',
		what: '/etc/nginx/conf.d/app.conf',
		bytes: 0,
		outcome: 'refused',
		why: '柵の外です'
	}
];

afterEach(() => cleanup());

describe('転送の列', () => {
	test('says plainly when nothing has moved', () => {
		render(TransferList, { lines: [] });
		expect(screen.getByText(i18n.t('transfer.empty'))).toBeTruthy();
	});

	test('keeps a line for everything that was given', () => {
		render(TransferList, { lines: LINES });
		expect(screen.getAllByRole('listitem').length).toBe(LINES.length);
	});

	test('puts the newest first, so what is happening is not buried', () => {
		render(TransferList, { lines: LINES });
		const first = screen.getAllByRole('listitem')[0];
		expect(first.textContent).toContain('/etc/nginx/conf.d/app.conf');
	});

	test('says who moved it, in words not only colour', () => {
		// **人（青）と AI（橙）は明るさがほぼ同じ**（差 9.5 / 255）——
		// 色が見えない人には届きません。
		render(TransferList, { lines: LINES });
		expect(screen.getAllByText(i18n.t('console.who.ai')).length).toBe(2);
		expect(screen.getAllByText(i18n.t('console.who.me')).length).toBe(1);
	});

	test('shows that something was stopped', () => {
		render(TransferList, { lines: LINES });
		expect(screen.getByText(i18n.t('transfer.refused'))).toBeTruthy();
	});

	test('says why it was stopped', () => {
		// **「止めました」だけでは、人は次に何をすればいいか分かりません。**
		render(TransferList, { lines: LINES });
		expect(screen.getByText('柵の外です')).toBeTruthy();
	});

	test('does not put a size on a line that was stopped', () => {
		// **「4.0 KB」と並んでいたら、人は動いたと読みます。**
		render(TransferList, { lines: LINES });
		const stopped = screen.getByText('/etc/nginx/conf.d/app.conf').closest('li');
		expect(stopped?.textContent).not.toMatch(/\d+(\.\d+)?\s?(B|KB|MB|GB)/);
	});

	test('marks the stopped line so it is not read as ordinary', () => {
		render(TransferList, { lines: LINES });
		const stopped = screen.getByText('/etc/nginx/conf.d/app.conf').closest('li');
		expect(stopped?.classList.contains('refused')).toBe(true);
		const fine = screen.getByText('/var/log/app/error.log').closest('li');
		expect(fine?.classList.contains('refused')).toBe(false);
	});

	test('lets the server paths be hidden', () => {
		// **道はサーバー側のパス。**接続先そのものです（禁止事項 4）。
		render(TransferList, { lines: LINES });
		for (const line of LINES) {
			expect(
				screen.getByText(line.what).hasAttribute('data-secret'),
				`${line.what} に伏せる印がありません`
			).toBe(true);
		}
	});

	test('says which way it went, for someone who cannot see the arrow', () => {
		render(TransferList, { lines: LINES });
		expect(screen.getAllByLabelText(i18n.t('transfer.up')).length).toBe(2);
		expect(screen.getAllByLabelText(i18n.t('transfer.down')).length).toBe(1);
	});
});
