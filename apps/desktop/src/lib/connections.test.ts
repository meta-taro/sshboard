/**
 * 接続の登録を止める条件。**理由を鍵で返す**ので、言語を変えても検査は同じ。
 *
 * ここが緩むと、**保存はできるが繋がらない登録**が一覧に増えます。
 */
import { describe, expect, test } from 'vitest';

import {
	CONNECTION_COLORS,
	CONNECTION_TAG_MAX_CHARS,
	emptyConnection,
	isConnectionTag,
	isPuttyKey,
	whyNotSavable,
	type Connection
} from './connections';

/** 保存できる最小の登録。**各テストはここから 1 か所だけ崩す。** */
function usable(overrides: Partial<Connection> = {}): Connection {
	return {
		id: 'app-prod',
		name: 'App (prod)',
		host: 'host.example.invalid',
		port: 22,
		user: 'deploy',
		...overrides
	};
}

describe('whyNotSavable', () => {
	test('lets a complete entry through', () => {
		expect(whyNotSavable(usable(), [])).toBeNull();
	});

	test('names the missing field rather than just refusing', () => {
		expect(whyNotSavable(usable({ id: '' }), [])).toEqual({ key: 'conn.err.id.empty' });
		expect(whyNotSavable(usable({ host: '' }), [])).toEqual({ key: 'conn.err.host' });
		expect(whyNotSavable(usable({ user: '' }), [])).toEqual({ key: 'conn.err.user' });
	});

	test('refuses an identifier that would not survive a file or a band line', () => {
		// 識別子はファイルにも帯にも出る。**空白や記号を許すと読めなくなる。**
		expect(whyNotSavable(usable({ id: 'app prod' }), [])).toEqual({ key: 'conn.err.id.chars' });
		expect(whyNotSavable(usable({ id: 'app/prod' }), [])).toEqual({ key: 'conn.err.id.chars' });
		expect(whyNotSavable(usable({ id: 'app.prod_2-a' }), [])).toBeNull();
	});

	test('refuses a port outside the range', () => {
		expect(whyNotSavable(usable({ port: 0 }), [])).toEqual({ key: 'conn.err.port' });
		expect(whyNotSavable(usable({ port: 65536 }), [])).toEqual({ key: 'conn.err.port' });
		expect(whyNotSavable(usable({ port: 65535 }), [])).toBeNull();
	});

	test('refuses a duplicate identifier instead of overwriting', () => {
		// **黙って上書きすると、人が登録したものが消える。**
		expect(whyNotSavable(usable(), ['app-prod'])).toEqual({
			key: 'conn.err.dup',
			id: 'app-prod'
		});
	});

	test('refuses a tag that is too long, counted in characters', () => {
		const tooLong = '本'.repeat(CONNECTION_TAG_MAX_CHARS + 1);
		expect(whyNotSavable(usable({ tag: tooLong }), [])).toEqual({
			key: 'conn.err.tag',
			max: CONNECTION_TAG_MAX_CHARS
		});
	});
});

describe('isConnectionTag', () => {
	test('counts characters, not bytes', () => {
		// 漢字 12 文字は 36 バイト。**バイトで数えると 4 文字で弾かれる。**
		expect(isConnectionTag('本'.repeat(CONNECTION_TAG_MAX_CHARS))).toBe(true);
		expect(isConnectionTag('本'.repeat(CONNECTION_TAG_MAX_CHARS + 1))).toBe(false);
	});

	test('treats no tag as fine', () => {
		expect(isConnectionTag(null)).toBe(true);
		expect(isConnectionTag(undefined)).toBe(true);
		expect(isConnectionTag('')).toBe(true);
	});
});

describe('emptyConnection', () => {
	test('starts with no write permission for the AI', () => {
		// **既定で書けるようにしない**（D22）。ここが崩れると囲いが意味を失う。
		expect(emptyConnection().write_roots).toEqual([]);
	});

	test('starts with ssh-agent, not a key file', () => {
		// 鍵のパスが空 = ssh-agent（D11 の推奨）。
		expect(emptyConnection().key_path).toBeUndefined();
	});
});

describe('isPuttyKey', () => {
	test('spots a .ppk so the person is warned before they hit the wall', () => {
		// この層は鍵を .ppk で持っている（D19）。**登録時に気づかせる。**
		expect(isPuttyKey('/keys/server.ppk')).toBe(true);
		expect(isPuttyKey('/keys/SERVER.PPK')).toBe(true);
		expect(isPuttyKey('/keys/id_ed25519')).toBe(false);
		expect(isPuttyKey(null)).toBe(false);
	});
});

describe('the mark colours', () => {
	test('are names, not hex, so the theme can pick light or dark', () => {
		for (const colour of CONNECTION_COLORS) {
			expect(colour).toMatch(/^[a-z]+$/);
		}
	});

	test('are enough to fill two even rows', () => {
		// 3 行になると高さを取りすぎる（人の指摘で 16 色に決めた）。
		expect(CONNECTION_COLORS.length).toBe(16);
	});
});
