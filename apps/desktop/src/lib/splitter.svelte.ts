/**
 * 2 面の仕切り。**掴んで動かし、ダブルクリックで定位置に戻す。**
 *
 * 幅は残します。**毎回そろえ直させない**のが目的なので、
 * 残さないとこの機能はほぼ無意味になります。
 */

/** 一覧が名前を失わない最小幅。 */
export const MIN_LIST_WIDTH = 140;

/** 入力側が潰れない上限（枠全体に対する割合）。 */
export const MAX_LIST_RATIO = 0.6;

/** 定位置。**ダブルクリックでここへ戻る。** */
export const DEFAULT_LIST_WIDTH = 240;

const WIDTH_KEY = 'sshboard-list-width';

/**
 * 枠の幅に収まる値へ丸める。**上限は枠に対する割合で決める。**
 *
 * 固定ピクセルで上限を決めると、窓を狭めたときに入力側が消える。
 */
export function clampListWidth(width: number, containerWidth: number): number {
	const max = Math.max(MIN_LIST_WIDTH, Math.floor(containerWidth * MAX_LIST_RATIO));
	return Math.min(Math.max(Math.round(width), MIN_LIST_WIDTH), max);
}

export function loadListWidth(): number {
	if (typeof localStorage === 'undefined') return DEFAULT_LIST_WIDTH;
	const stored = Number(localStorage.getItem(WIDTH_KEY));
	return Number.isFinite(stored) && stored >= MIN_LIST_WIDTH ? stored : DEFAULT_LIST_WIDTH;
}

export function saveListWidth(width: number): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(WIDTH_KEY, String(Math.round(width)));
}

/* ------------------------------------------------------------------ *
 * 割合で分ける側（ファイル 2 ペイン）
 *
 * 接続管理は「一覧の固定幅」だが、こちらは**左右の取り分**なので割合で持つ。
 * 窓の大きさが変わっても取り分が保たれる。ピクセルで持つと、
 * 窓を広げたときに片側だけが伸びる。
 * ------------------------------------------------------------------ */

/** 片側がこれ以下になると、名前もパスも読めなくなる。 */
export const MIN_PANE_RATIO = 0.2;

/** 定位置。**ダブルクリックでここへ戻る。** */
export const DEFAULT_PANE_RATIO = 0.5;

const PANE_RATIO_KEY = 'sshboard-pane-ratio';

/**
 * 割合を読める範囲へ丸める。
 *
 * **数でないものは定位置へ倒します。**落として画面を消すより、
 * 真ん中に戻る方がまだ直せる。
 */
export function clampPaneRatio(ratio: number): number {
	if (!Number.isFinite(ratio)) return DEFAULT_PANE_RATIO;
	return Math.min(Math.max(ratio, MIN_PANE_RATIO), 1 - MIN_PANE_RATIO);
}

export function loadPaneRatio(): number {
	if (typeof localStorage === 'undefined') return DEFAULT_PANE_RATIO;
	const stored = Number(localStorage.getItem(PANE_RATIO_KEY));
	return Number.isFinite(stored) && stored > 0 ? clampPaneRatio(stored) : DEFAULT_PANE_RATIO;
}

export function savePaneRatio(ratio: number): void {
	if (typeof localStorage === 'undefined') return;
	// **壊れた値を残さない。**読み出し側だけで守ると、書いた値と読む値がずれる。
	localStorage.setItem(PANE_RATIO_KEY, String(clampPaneRatio(ratio)));
}
