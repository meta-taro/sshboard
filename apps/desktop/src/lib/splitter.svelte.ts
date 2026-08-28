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
