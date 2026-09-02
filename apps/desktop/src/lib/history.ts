/**
 * たどった道を覚える。**戻る／進むのため。**
 *
 * α を実機で触った人から「**戻るをマウスでしても戻らない**」。
 * ファイルを見て回る道具で、戻れないのは効きます。
 *
 * **中身は書き換えません**（coding-style）。毎回新しいものを返します。
 * 左右のペインが同じ型を別々に持つので、片方の操作でもう片方が
 * 黙って変わると追えなくなります。
 */

export type History = {
	/** たどった順。 */
	readonly entries: readonly string[];
	/** いまどこに居るか（`entries` の位置）。 */
	readonly index: number;
};

/**
 * 覚えておく上限。
 *
 * **際限なく持たない。**長く使うほど太る一方になります。
 * 200 は「1 日触っても戻り切れる」程度の目安で、実測で決めたものではありません。
 */
const LIMIT = 200;

export function createHistory(initial: string): History {
	return { entries: [initial], index: 0 };
}

export function current(history: History): string {
	return history.entries[history.index] ?? '';
}

export function canBack(history: History): boolean {
	return history.index > 0;
}

export function canForward(history: History): boolean {
	return history.index < history.entries.length - 1;
}

/**
 * 別の所へ移る。
 *
 * **戻ったあとに別の所へ行くと、先の分は捨てます**（ブラウザと同じ）。
 * 残すと「進む」がどこへ行くのか読めなくなります。
 *
 * **同じ所へ続けて行っても増やしません。**増やすと、再読み込みのたびに
 * 履歴が伸びて「戻る」を何度押しても同じ場所になります。
 */
export function visit(history: History, path: string): History {
	if (current(history) === path) return history;

	const kept = history.entries.slice(0, history.index + 1);
	const entries = [...kept, path];
	// 上限を超えたら**古い方から捨てる。**
	const trimmed = entries.length > LIMIT ? entries.slice(entries.length - LIMIT) : entries;
	return { entries: trimmed, index: trimmed.length - 1 };
}

/** **端で押しても落ちません。**そこに留まります。 */
export function back(history: History): History {
	if (!canBack(history)) return history;
	return { entries: history.entries, index: history.index - 1 };
}

export function forward(history: History): History {
	if (!canForward(history)) return history;
	return { entries: history.entries, index: history.index + 1 };
}
