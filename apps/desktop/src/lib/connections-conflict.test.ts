/**
 * **編集している間に、外からファイルが変わったら知らせる**（PdM の指摘・2026-09-28）。
 *
 * 一覧は人（GUI）と AI（MCP）の両方が書き換えます。画面は読み直しても
 * 編集中の下書き（`draft`）を触らないので、**画面が黙って消される心配はありません。**
 * 残っていたのは逆向きです ——
 *
 * 1. 人が接続を開いて直し始める
 * 2. その間に、外から（AI・別の道具・エディタ）同じ接続が書き換わる
 * 3. 人が保存を押す → **相手の変更が、人の下書きで消える**
 *
 * **消えたことは、どちらにも見えません。**だから保存の前に知らせます。
 */

import { describe, expect, test } from 'vitest';
import { externalConflict } from './connections-conflict';
import type { Connection } from './connections';

function entry(over: Partial<Connection> = {}): Connection {
	return {
		id: 'web',
		name: 'Web',
		host: 'example.invalid',
		port: 22,
		user: 'ops',
		key_path: null,
		keyring_passphrase_ref: null,
		fingerprint: null,
		known_hosts: null,
		color: null,
		tag: null,
		write_roots: [],
		...over
	};
}

describe('編集中の接続と、読み直した一覧を見比べる', () => {
	test('returns nothing while the file still says what it said', () => {
		const base = entry();

		expect(externalConflict(base, [entry()])).toBe(null);
	});

	test('returns nothing for a brand new draft, which has nothing to compare', () => {
		expect(externalConflict(null, [entry()])).toBe(null);
	});

	test('notices a field the file changed underneath the editor', () => {
		const base = entry();

		expect(externalConflict(base, [entry({ port: 2222 })])).toBe('changed');
	});

	test('notices the connection being removed from the file', () => {
		const base = entry();

		expect(externalConflict(base, [entry({ id: 'other' })])).toBe('gone');
	});

	test('notices a change in the write roots, which is what the AI may write', () => {
		// **囲い（D22）が外から広げられたら、それは知らせるべき変化。**
		const base = entry({ write_roots: ['/srv/app'] });

		expect(externalConflict(base, [entry({ write_roots: ['/srv/app', '/etc'] })])).toBe('changed');
	});

	test('does not mistake an absent field for a changed one', () => {
		// Rust 側は省略できる項目を出さないことがある。**null と未設定は同じ意味。**
		const base = entry({ tag: null });
		const incoming = entry();
		delete incoming.tag;

		expect(externalConflict(base, [incoming])).toBe(null);
	});

	test('notices a field that only the new version has', () => {
		// **比べる項目を手で並べていると、後から足した項目を見落とす。**
		// ここが落ちたら、比較は列挙式に戻っている。
		const base = entry();
		const incoming = { ...entry(), become: 'ask' } as Connection & { become: string };

		expect(externalConflict(base, [incoming])).toBe('changed');
	});
});
