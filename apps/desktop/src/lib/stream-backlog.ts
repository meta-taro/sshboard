/**
 * 直近の出力を覚えておく（Issue #14 / #11）。
 *
 * **端末の面は、そのタブを見ている間しか存在しません。**
 * 貼り先の要素が `{#if view === 'console'}` の中にあるためで、
 * 別のタブに居る間に届いた分は**どこへも書かれず**、
 * 見に行くと**まっさらな面**ができます。
 *
 * 実機では、AI が端末で打っている間ずっと人は別のタブに居て、
 * **見に行ったときには何も無い**という形で出ました。承認のダイアログには
 * 「**打った内容は画面にそのまま出ますし**」と書いてあります ——
 * **同意の前提が実態と違っていました。**
 *
 * ここが覚えておき、面ができた時点で書き戻します。
 *
 * **ディスクには 1 バイトも書きません。**出力には接続先も打鍵の中身も乗るので、
 * **残す場所を増やさない**のが要点です（PRD §8）。
 */

/**
 * 画面の中に持つ上限。
 *
 * **青天井にしない**（`tail -f` を流したままにすると何 MB でも溜まる）。
 * 端末の見えている高さ ＋ 少しの巻き戻しが戻れば足りるので、256 KB。
 */
export const MAX_BACKLOG_BYTES = 256 * 1024;

/** 覚えている塊と、その合計。**数えなおさないため合計を持ちます。** */
export interface Backlog {
	readonly chunks: readonly (readonly number[])[];
	readonly bytes: number;
}

export function emptyBacklog(): Backlog {
	return { chunks: [], bytes: 0 };
}

/**
 * 1 つ覚える。**渡されたものは書き換えません**（coding-style）。
 *
 * 上限を超えたら**古い方から捨てます。**人が見たいのは直前の出力です。
 */
export function remember(backlog: Backlog, chunk: readonly number[]): Backlog {
	// 1 つで上限を超える塊が来ることがあります（大きなファイルを `cat` した等）。
	// **丸ごと捨てると直前の出力まで消える**ので、末尾だけ残します。
	const incoming =
		chunk.length > MAX_BACKLOG_BYTES ? chunk.slice(chunk.length - MAX_BACKLOG_BYTES) : chunk;

	let chunks = [...backlog.chunks, incoming];
	let bytes = backlog.bytes + incoming.length;

	// **前から落とす。**`shift` は元の配列を書き換えるので使いません。
	let dropped = 0;
	while (bytes > MAX_BACKLOG_BYTES && dropped < chunks.length) {
		bytes -= chunks[dropped].length;
		dropped += 1;
	}
	if (dropped > 0) chunks = chunks.slice(dropped);

	return { chunks, bytes };
}

/** 書き戻す 1 本にまとめる。**空なら空**（画面を汚さない）。 */
export function replay(backlog: Backlog): number[] {
	return backlog.chunks.flatMap((chunk) => [...chunk]);
}

/**
 * まだ 1 本も繋いでいない間の出力の置き場。
 *
 * **捨てません。**繋ぐ前にも出るものがあります（繋げなかった理由など）。
 */
export const BEFORE_CONNECTING = '';

/**
 * 接続ごとの控え（D60・2026-10-02）。
 *
 * **なぜ分けたか**: 端末が接続ごとに持てるようになったので、控えを 1 本で持つと
 * **タブを戻したときに別のサーバーの出力が出ます。**
 * 人は「いま打ったもの」と「別の台で打ったもの」を見分けられません。
 */
export type Backlogs = ReadonlyMap<string, Backlog>;

export function emptyBacklogs(): Backlogs {
	return new Map();
}

/** イベントの `connection`（繋ぐ前は `null`）を鍵にする。 */
export function keyFor(connection: string | null | undefined): string {
	return connection ?? BEFORE_CONNECTING;
}

/** 1 つ覚える。**渡された地図は書き換えません**（coding-style）。 */
export function rememberFor(
	all: Backlogs,
	connection: string,
	chunk: readonly number[]
): Backlogs {
	const next = new Map(all);
	next.set(connection, remember(all.get(connection) ?? emptyBacklog(), chunk));
	return next;
}

/** その接続の分だけを書き戻す 1 本。**知らない接続なら空**（別の分で埋めない）。 */
export function replayFor(all: Backlogs, connection: string | null): number[] {
	if (connection === null) return [];
	const held = all.get(connection);
	return held ? replay(held) : [];
}

/**
 * 切れた接続の分を捨てる。
 *
 * **残すと、繋ぎ直したときに前回の出力が混ざって出ます。**
 */
export function forget(all: Backlogs, connection: string): Backlogs {
	if (!all.has(connection)) return all;
	const next = new Map(all);
	next.delete(connection);
	return next;
}
