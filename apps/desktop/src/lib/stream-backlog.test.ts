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

import {
	BEFORE_CONNECTING,
	emptyBacklog,
	emptyBacklogs,
	forget,
	MAX_BACKLOG_BYTES,
	remember,
	rememberFor,
	replay,
	replayFor,
	type Backlog
} from './stream-backlog';

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

describe('接続ごとの控え（D60）', () => {
	test('keeps each connection apart', () => {
		// **一番危ない食い違い。**端末が接続ごとに持てるようになったので（D60）、
		// 控えを 1 本で持つと、タブを戻したとき**別のサーバーの出力が出ます。**
		let all = emptyBacklogs();
		all = rememberFor(all, 'first', [65]);
		all = rememberFor(all, 'second', [66]);

		expect(replayFor(all, 'first')).toEqual([65]);
		expect(replayFor(all, 'second')).toEqual([66]);
	});

	test('returns nothing for a connection it never saw', () => {
		// **空を返す。**別の接続の分で埋めない。
		expect(replayFor(emptyBacklogs(), 'nobody')).toEqual([]);
		expect(replayFor(emptyBacklogs(), null)).toEqual([]);
	});

	test('does not change the map it was given', () => {
		const before = emptyBacklogs();
		const after = rememberFor(before, 'first', [65]);

		expect(before.size).toBe(0);
		expect(after.size).toBe(1);
	});

	test('forgets a connection that was closed', () => {
		// **残すと、繋ぎ直したときに前回の出力が混ざって出ます。**
		// 人は「いま打ったもの」と「前に打ったもの」を見分けられません。
		let all = rememberFor(emptyBacklogs(), 'first', [65]);
		all = forget(all, 'first');

		expect(replayFor(all, 'first')).toEqual([]);
	});

	test('keeps the output from before anything was connected', () => {
		// 繋ぐ前にも出るものがあります（接続の失敗など）。**捨てません。**
		const all = rememberFor(emptyBacklogs(), BEFORE_CONNECTING, [65]);

		expect(replayFor(all, BEFORE_CONNECTING)).toEqual([65]);
	});

	test('the per-connection tail is bounded just like the single one', () => {
		let all = emptyBacklogs();
		const chunk = Array.from({ length: 1024 }, () => 65);
		for (let i = 0; i < 400; i += 1) all = rememberFor(all, 'first', chunk);

		expect(replayFor(all, 'first').length).toBeLessThanOrEqual(MAX_BACKLOG_BYTES);
		expect(replayFor(all, 'first').length).toBeGreaterThan(0);
	});
});
