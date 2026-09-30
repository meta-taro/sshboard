/**
 * **［↑］が「一つ上」へ行く**（Issue #8・2026-09-30 に実機で再現）。
 *
 * 実機の言葉 ——
 *
 * > **上押したのに元の表示じゃないですね**
 * > **ルートに移動してます**
 *
 * リモート側は `.`（入った所）から始まります。`app` へ入ると `app` になり、
 * そこで［↑］を押すと **`/`（根）へ飛んでいました。**
 *
 * ```
 * parentOf("app") → "/" が 1 つも無い → cut = -1 → **cut <= 0 なので "/" を返す**
 * ```
 *
 * **絶対パスのつもりで書かれていて、相対パスが根へ落ちていました。**
 * 根はコンテナや実機の `/` なので、**まったく別の場所の一覧が出ます。**
 * 「画面が実態とずれる」の、いちばん分かりやすい形です。
 */
import { describe, expect, test } from 'vitest';

import { parentOf } from './session.svelte';

describe('絶対パス', () => {
	test('climbs one level', () => {
		expect(parentOf('/srv/app/logs')).toBe('/srv/app');
	});

	test('stops at the root instead of going above it', () => {
		expect(parentOf('/srv')).toBe('/');
		expect(parentOf('/')).toBe('/');
	});

	test('ignores a trailing slash', () => {
		expect(parentOf('/srv/app/')).toBe('/srv');
	});
});

describe('相対パス（入った所からの道）', () => {
	test('goes back to where we started, not to the filesystem root', () => {
		// **ここが実機で踏まれた所。**"app" の上は "." であって "/" ではない。
		expect(parentOf('app')).toBe('.');
	});

	test('climbs one level inside a relative path', () => {
		expect(parentOf('app/logs')).toBe('app');
		expect(parentOf('app/logs/2026')).toBe('app/logs');
	});

	test('climbs above the starting point one step at a time', () => {
		// **入った所より上へ行きたいことはある。**ただし根へ飛ばない。
		expect(parentOf('.')).toBe('..');
		expect(parentOf('..')).toBe('../..');
	});

	test('ignores a trailing slash', () => {
		expect(parentOf('app/logs/')).toBe('app');
	});
});
