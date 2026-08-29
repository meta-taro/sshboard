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
	keyNotice,
	whyNotSavable,
	type Connection,
	type KeyReport
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

describe('keyNotice', () => {
	// **拡張子で判定しない**（D28）。判定は Rust が中身を見て返し、
	// ここは「その結果をどう見せるか」だけを決める。
	const report = (over: Partial<KeyReport> = {}): KeyReport => ({
		readable: true,
		usable: true,
		needsPassphrase: false,
		unsupportedEncryption: false,
		format: 'OpenSSH',
		...over
	});

	test('says nothing until a key has actually been chosen', () => {
		expect(keyNotice('', report()).tone).toBe('none');
		expect(keyNotice(null, report()).tone).toBe('none');
	});

	test('separates "the file is not there" from "the format is wrong"', () => {
		// 一緒にすると、人は打ち間違いを疑わずに鍵を作り直しはじめる。
		expect(keyNotice('/keys/x', report({ readable: false }))).toEqual({
			tone: 'error',
			key: 'conn.key.missing',
			format: ''
		});
	});

	test('names the format when the file cannot be used to authenticate', () => {
		expect(keyNotice('/keys/x.pub', report({ usable: false, format: 'public key' }))).toEqual({
			tone: 'error',
			key: 'conn.key.unusable',
			format: 'public key'
		});
	});

	test('treats a PuTTY key as ordinary — no conversion is asked for', () => {
		// **ここが D19 との違いです。**以前は .ppk を見ると
		// 「puttygen で変換してください」と出していた。russh はそのまま読める。
		expect(keyNotice('/keys/x.ppk', report({ format: 'PuTTY (PPK v3)' }))).toEqual({
			tone: 'info',
			key: 'conn.key.ok',
			format: 'PuTTY (PPK v3)'
		});
	});

	test('gives its own reason when the format is readable but the cipher is not', () => {
		// 「秘密鍵を指してください」は的外れ。**指している。**
		expect(
			keyNotice('/keys/x', report({ usable: false, unsupportedEncryption: true, format: 'PKCS#8' }))
		).toEqual({ tone: 'error', key: 'conn.key.unsupported', format: 'PKCS#8' });
	});

	test('says up front that a passphrase will be asked for', () => {
		// 繋ぐ瞬間に初めて聞かれると、人は「失敗した」と受け取る。
		expect(keyNotice('/keys/x', report({ needsPassphrase: true }))).toEqual({
			tone: 'info',
			key: 'conn.key.passphrase',
			format: 'OpenSSH'
		});
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
