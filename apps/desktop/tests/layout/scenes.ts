import type { Page } from '@playwright/test';

import { emitEvent, fakeTauri, opened, type Replies } from './fake-tauri';

/**
 * **見栄えを測るための場面**（2026-10-26）。
 *
 * 崩れは「中身が多いとき」「何も無いとき」「狭いとき」に出てきました。
 * 架空の接続だけで、その 2 つの極を作ります（**接続先は架空**・禁止事項 4）。
 */
export type Scene = 'fresh' | 'busy';

const NAMES = [
	'kingdom 001',
	'kingdom 002',
	'L5 web',
	'L5 db',
	'dk006',
	'dk008',
	'dk009',
	'batch runner',
	'stage',
	'mail'
];

function registered() {
	return NAMES.map((name, n) => ({
		id: `c${n}`,
		name,
		host: 'example.invalid',
		port: 22,
		user: 'demo',
		color: ['teal', 'blue', 'orange', null][n % 4],
		tag: n % 3 === 0 ? 'prod' : null,
		write_roots: n === 0 ? ['/srv/app'] : []
	}));
}

function listing(prefix: string) {
	return Array.from({ length: 24 }, (_, n) => ({
		name: n < 5 ? `${prefix}-folder-${n}` : `${prefix}-file-${n}.log`,
		isDir: n < 5,
		size: n * 1234,
		aiWritable: false
	}));
}

function replies(scene: Scene): Replies {
	const base: Replies = {
		connections_list: registered(),
		transfers_recent: [],
		diagnostics_recent: [],
		console_list: [],
		session_status: { open: [], active: null },
		local_list_dir: { path: '/home/demo', parent: '/home', entries: listing('local') }
	};
	if (scene === 'fresh') return base;
	return {
		...base,
		session_status: {
			open: [opened('c0', 'kingdom 001'), opened('c4', 'dk006')],
			active: 'c0'
		},
		console_holder: { holder: 'human', connection: 'c0' },
		console_list: [{ id: 1, connection: 'c0', holder: 'human' }],
		remote_list_dir: listing('remote'),
		transfers_recent: Array.from({ length: 6 }, (_, n) => ({
			seq: n,
			actor: n % 2 ? 'ai' : 'human',
			direction: n % 2 ? 'down' : 'up',
			what: `/srv/app/releases/build-${n}.tar.gz`,
			bytes: 4096 * n,
			outcome: n === 5 ? 'refused' : 'done',
			why: n === 5 ? '柵の外です' : null
		})),
		diagnostics_recent: Array.from({ length: 40 }, (_, n) => ({
			seq: n,
			atMs: n * 150,
			level: ['info', 'warn', 'error'][n % 3],
			stage: 'exec',
			connection: n % 2 ? 'c0' : 'c4',
			message: `記録 ${n}`,
			hint: n % 3 === 2 ? '次にやること' : undefined
		}))
	};
}

export async function stage(
	page: Page,
	scene: Scene,
	options: { locale: string; theme: string; width: number; height: number }
): Promise<void> {
	await page.addInitScript(
		([locale, theme]) => {
			try {
				localStorage.setItem('sshboard-locale', locale);
				localStorage.setItem('sshboard-theme', theme);
			} catch {
				// 保存できなくても既定で測れる。
			}
		},
		[options.locale, options.theme]
	);
	await fakeTauri(page, replies(scene));
	await page.setViewportSize({ width: options.width, height: options.height });
	await page.goto('/');
	await page.locator('header').waitFor();
	if (scene === 'busy') {
		for (let n = 0; n < 30; n += 1) {
			await emitEvent(page, 'band://line', {
				seq: n,
				tag: n % 2 ? '[AI]' : '[Human]',
				text: `line ${n}`,
				rendered: `${n % 2 ? '[AI]' : '[Human]'} line ${n}`
			});
		}
	}
}
