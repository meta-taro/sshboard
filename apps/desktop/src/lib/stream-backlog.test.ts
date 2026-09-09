/**
 * 直近の出力を覚えておく（Issue #14 / #11）。
 *
 * 端末の面（`consoleTerm`）は、**そのタブを見ている間しか存在しません。**
 * 貼り先の要素が `{#if view === 'console'}` の中にあるためです。
 *
 * | そのとき | 端末の面 | 結果 |
 * |---|---|---|
 * | 人が別のタブに居る | **無い** | **書かれない。どこへも残らない** |
 * | 人が端末タブへ移る | **作り直される** | **まっさら。それまでの分は無い** |
 *
 * 実機ではこう出ました —— AI が端末で打っている間、人は［接続］タブに居て、
 * **見に行ったときには何も無い。**
 *
 * > あなたは裏で操作しているため、わたしにはその情報を sshboard で確認できていません。
 *
 * 承認のダイアログには「**打った内容は画面にそのまま出ますし**」と書いてあります。
 * **同意の前提が実態と違っていました。**
 *
 * ここが覚えておき、面ができた時点で書き戻します。
 */
import { describe, expect, test } from 'vitest';

import { emptyBacklog, MAX_BACKLOG_BYTES, remember, replay, type Backlog } from './stream-backlog';

/** `size` バイトの塊。中身は何でもよいので、位置が分かる値を入れる。 */
function chunk(size: number, fill: number): number[] {
	return new Array(size).fill(fill);
}

describe('stream-backlog', () => {
	test('remembers what arrived while nobody was looking', () => {
		let held: Backlog = emptyBacklog();

		held = remember(held, [104, 105]);
		held = remember(held, [10]);

		expect(replay(held)).toEqual([104, 105, 10]);
	});

	test('never changes what it was given', () => {
		// **既存のものを書き換えない**（coding-style）。
		// 書き換えると、描き直しの途中で中身がすり替わります。
		const first = emptyBacklog();
		const incoming = [1, 2, 3];

		const second = remember(first, incoming);

		expect(replay(first)).toEqual([]);
		expect(incoming).toEqual([1, 2, 3]);
		expect(second).not.toBe(first);
	});

	test('drops the oldest instead of growing without a limit', () => {
		// **青天井にしない。**`tail -f` を流したまま放っておくと、
		// 画面の中に何 MB でも溜まります。
		let held: Backlog = emptyBacklog();
		const each = MAX_BACKLOG_BYTES / 4;

		for (const fill of [1, 2, 3, 4, 5]) {
			held = remember(held, chunk(each, fill));
		}

		const kept = replay(held);
		expect(kept.length).toBeLessThanOrEqual(MAX_BACKLOG_BYTES);
		// **新しい方を残す。**人が見たいのは直前の出力です。
		expect(kept[kept.length - 1]).toBe(5);
		expect(kept.includes(1)).toBe(false);
	});

	test('keeps the tail when a single chunk is bigger than the limit', () => {
		// 1 つの塊が上限を超えることがあります（大きなファイルを `cat` した等）。
		// **丸ごと捨てると、直前の出力まで消えます。**末尾を残します。
		let held: Backlog = emptyBacklog();

		held = remember(held, [...chunk(MAX_BACKLOG_BYTES * 2, 7), 9]);

		const kept = replay(held);
		expect(kept.length).toBeLessThanOrEqual(MAX_BACKLOG_BYTES);
		expect(kept[kept.length - 1]).toBe(9);
	});

	test('replays nothing when nothing has arrived', () => {
		// **空を書き戻して、画面を汚さない。**
		expect(replay(emptyBacklog())).toEqual([]);
	});
});
