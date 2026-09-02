import { describe, expect, test } from 'vitest';

import { mirrorMove, type Move } from './sync-browse';

/**
 * **WinSCP と同じ考え方**にしています。
 * 絶対パスを合わせるのではなく、**同じ「動き」をもう片方でもする。**
 * 左が `/home/me`・右が `/srv/app` のとき、左で `logs` へ入ったら
 * 右も `logs` へ入る（`/srv/app/logs`）。
 */
describe('同期移動 — 同じ動きをもう片方でもする', () => {
	test('中へ入ったら、相手も同じ名前の中へ入る', () => {
		const move: Move = { kind: 'into', name: 'logs' };
		expect(mirrorMove('/srv/app', move, '/')).toBe('/srv/app/logs');
	});

	test('上がったら、相手も上がる', () => {
		expect(mirrorMove('/srv/app/logs', { kind: 'up' }, '/')).toBe('/srv/app');
	});

	test('相手が根まで来ていたら、それ以上は上がらない', () => {
		// **落とさない。**片方が浅い所に居るのは普通に起きます。
		expect(mirrorMove('/', { kind: 'up' }, '/')).toBe('/');
	});

	test('Windows のパスでも同じように動く', () => {
		// 手元が Windows・相手が Linux という組み合わせが普通に起きます。
		expect(mirrorMove('C:\\work', { kind: 'into', name: 'logs' }, '\\')).toBe('C:\\work\\logs');
		expect(mirrorMove('C:\\work\\logs', { kind: 'up' }, '\\')).toBe('C:\\work');
	});

	test('ドライブの根より上へは行かない', () => {
		expect(mirrorMove('C:\\', { kind: 'up' }, '\\')).toBe('C:\\');
	});

	test('直接パスを打った動きは、相手に伝えない', () => {
		// **絶対パスは相手にとって意味が無い。**
		// `/home/me/dev` を打った動きを右へ渡しても、そんな場所はありません。
		expect(mirrorMove('/srv/app', { kind: 'jump' }, '/')).toBe(null);
	});
});
