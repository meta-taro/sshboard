/**
 * 接続の並べ替え（dbboard から移植）。
 *
 * **並び順 ＝ `connections.toml` の `[[connections]]` の順**です。
 * 別の項目で持たないので、**並べ替えとはファイルを書き換えること**になります。
 *
 * 計算をここへ出しているのは、**端（先頭・末尾）の場合を試験できるようにする**ため。
 * 画面の中に書くと、そこだけ確かめられません。
 */

/**
 * ▲ ▼ を押したとき、どこへ行くか。**行き先が無ければ `null`。**
 *
 * `null` がボタンを無効にする根拠になります。
 *
 * @param index いまの位置
 * @param delta ▲ なら -1、▼ なら +1
 * @param length 何件あるか
 */
export function moveTarget(index: number, delta: number, length: number): number | null {
	if (index < 0 || index >= length) return null;
	const target = index + delta;
	if (target < 0 || target >= length) return null;
	return target;
}

/**
 * 掴んで落としたとき、どこへ行くか。**動かないなら `null`。**
 *
 * ▲▼ とは問いが違います。**掴んだ方は「隙間」を指します** —
 * 行と行の間で、0〜件数の範囲。両端も名指しできるようにするためです。
 *
 * **掴んでいる行は、まだ一覧の中に居ます。**抜き取るとそれより後ろの隙間が
 * 1 つ詰まるので、**自分より下の隙間は 1 つ大きく数えられています。**
 * ここがずれの正体で、だから関数に切り出しています。
 *
 * @param from 掴んだ行の、いまの位置
 * @param gap 落とした隙間（0〜件数）
 * @param length 何件あるか
 */
export function dropTarget(from: number, gap: number, length: number): number | null {
	if (from < 0 || from >= length) return null;
	if (gap < 0 || gap > length) return null;
	const target = gap > from ? gap - 1 : gap;
	// **その行の両隣の隙間は、どちらもその行自身を指します。**
	// そこへ落とすのは誤りではなく、ただ「動かない」だけ。
	// **同じ内容を書き直してファイルを触らない。**
	if (target === from) return null;
	return target;
}

/**
 * 実際に並べ替えた一覧を返す。**元の配列は書き換えません**（coding-style）。
 */
export function reorder<T>(items: readonly T[], from: number, to: number): T[] {
	const next = [...items];
	const [moved] = next.splice(from, 1);
	next.splice(to, 0, moved);
	return next;
}

/**
 * いま指がどの隙間の上に居るか。**各行の縦の中点**から決めます。
 *
 * 行の境目ではなく中点で切るのは、**境目で答えが震えないため**。
 * 境目で切ると、線が入った瞬間に行が動いて、また境目に戻る、を繰り返します。
 *
 * @param y 指の位置（`midpoints` と同じ座標系）
 * @param midpoints 各行の縦の中点（並んでいる順）
 */
export function gapForPointer(y: number, midpoints: readonly number[]): number {
	let gap = 0;
	while (gap < midpoints.length && midpoints[gap] < y) gap += 1;
	return gap;
}
