import { describe, expect, test } from 'vitest';

import { defaultBundleName } from './bundle-name';

describe('書き出すファイルの既定の名前', () => {
	test('日時が入る', () => {
		// 2026-09-03 15:30:45（現地時刻）
		const name = defaultBundleName(new Date(2026, 8, 3, 15, 30, 45));
		expect(name).toBe('sshboard-20260903-153045.sshbx');
	});

	test('1 桁の月日時分秒を 0 で埋める', () => {
		// 埋めないと `2026-9-3-9-5-1` のように桁が揃わず、**並べたときに順序が崩れます。**
		const name = defaultBundleName(new Date(2026, 0, 2, 3, 4, 5));
		expect(name).toBe('sshboard-20260102-030405.sshbx');
	});

	test('拡張子は .sshbx', () => {
		expect(defaultBundleName()).toMatch(/\.sshbx$/);
	});

	test('続けて書き出しても、秒が違えば名前が変わる', () => {
		// **同じ分に 2 回出すのは普通に起きます**（選び直して出し直す）。
		const a = defaultBundleName(new Date(2026, 8, 3, 15, 30, 45));
		const b = defaultBundleName(new Date(2026, 8, 3, 15, 30, 46));
		expect(a).not.toBe(b);
	});
});
