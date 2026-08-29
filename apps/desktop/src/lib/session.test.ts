/**
 * パスと大きさの扱い。**画面を開かなくても確かめられる部分。**
 *
 * ここが崩れると、**上げ先が 1 つ上のディレクトリになる**といった、
 * 気づきにくく取り返しのつかない壊れ方をします。
 */
import { describe, expect, test } from 'vitest';

import { baseName, humanSize, joinPath, localJoin, parentOf } from './session.svelte';

describe('localJoin', () => {
	test('joins a unix path with a single separator', () => {
		expect(localJoin('/Users/someone/build', 'app.tar.gz')).toBe('/Users/someone/build/app.tar.gz');
		expect(localJoin('/Users/someone/build/', 'app.tar.gz')).toBe(
			'/Users/someone/build/app.tar.gz'
		);
	});

	test('keeps the Windows separator instead of mixing the two', () => {
		// **Windows は配布対象**（PRD §7）。`C:\build/app` のような混ざり方をすると、
		// 落とし先が人の見ている場所と違って見える。
		expect(localJoin('C:\\build', 'app.tar.gz')).toBe('C:\\build\\app.tar.gz');
		expect(localJoin('C:\\build\\', 'app.tar.gz')).toBe('C:\\build\\app.tar.gz');
	});

	test('picks one separator when the path already holds both', () => {
		// WSL やツールの都合で `/` 混じりの Windows パスが来ることがある。
		// **どちらか一方に決める**（決めないと `\/` のような繋ぎ目を作る）。
		expect(localJoin('C:\\build/out', 'app.tar.gz')).toBe('C:\\build/out/app.tar.gz');
		expect(localJoin('C:/build', 'app.tar.gz')).toBe('C:/build/app.tar.gz');
	});
});

describe('parentOf', () => {
	test('goes up exactly one level', () => {
		expect(parentOf('/srv/app/releases')).toBe('/srv/app');
	});

	test('stops at the root instead of going above it', () => {
		// **上へ行き過ぎない。**`/` の親を空にすると、相対パスとして扱われる。
		expect(parentOf('/srv')).toBe('/');
		expect(parentOf('/')).toBe('/');
		expect(parentOf('')).toBe('/');
	});

	test('ignores a trailing slash', () => {
		expect(parentOf('/srv/app/')).toBe('/srv');
	});
});

describe('joinPath', () => {
	test('never doubles the separator', () => {
		expect(joinPath('/srv/app', 'a.tar.gz')).toBe('/srv/app/a.tar.gz');
		expect(joinPath('/srv/app/', 'a.tar.gz')).toBe('/srv/app/a.tar.gz');
		expect(joinPath('/', 'a.tar.gz')).toBe('/a.tar.gz');
	});
});

describe('baseName', () => {
	test('takes the last segment of a unix path', () => {
		expect(baseName('/Users/someone/build/app.tar.gz')).toBe('app.tar.gz');
	});

	test('understands a Windows path too', () => {
		// **Windows は配布対象**（PRD §7）。手元のパスは `\` で来る。
		expect(baseName('C:\\build\\app.tar.gz')).toBe('app.tar.gz');
	});

	test('returns the whole string when there is no separator', () => {
		expect(baseName('app.tar.gz')).toBe('app.tar.gz');
	});
});

describe('humanSize', () => {
	test('shows bytes exactly, without a decimal point', () => {
		// 1 バイト単位で意味がある場面がある（空ファイルかどうか等）。
		expect(humanSize(0)).toBe('0 B');
		expect(humanSize(1023)).toBe('1023 B');
	});

	test('divides by 1024, not 1000', () => {
		expect(humanSize(1024)).toBe('1.0 KB');
		expect(humanSize(1024 * 1024)).toBe('1.0 MB');
		expect(humanSize(1536)).toBe('1.5 KB');
	});

	test('stops at the largest unit it knows rather than losing the number', () => {
		expect(humanSize(1024 ** 5)).toBe('1024.0 TB');
	});
});
