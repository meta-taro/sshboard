/**
 * 編集している間に、外から一覧が書き換わったかを見る。
 *
 * **なぜ要るか**: 一覧は人（GUI）と AI（MCP）の両方が書き換えます。画面は読み直しても
 * 編集中の下書きを触らないので、**画面側が黙って消される心配はありません。**
 * 残っていたのは逆向き —— 人が編集している間に外から書き換わり、人が保存を押すと、
 * **相手の変更が人の下書きで消えます。**消えたことは、どちらにも見えません。
 */

import type { Connection } from './connections';

/** 編集中の接続が、ファイル側でどうなっていたか。 */
export type Conflict = 'changed' | 'gone' | null;

/**
 * 編集を始めたときの姿（`baseline`）と、読み直した一覧を見比べる。
 *
 * `baseline` が無いのは新規の下書きで、**比べる相手が居ません**
 * （同じ `id` が先に取られていたら、保存側が断ります）。
 */
export function externalConflict(baseline: Connection | null, items: Connection[]): Conflict {
	if (!baseline) return null;

	const now = items.find((item) => item.id === baseline.id);
	if (!now) return 'gone';

	return sameShape(baseline, now) ? null : 'changed';
}

/**
 * 2 つを見比べる。
 *
 * **項目を並べません。**並べると、後から足した項目（`become` のような）が
 * 比較から漏れ、**「変わっていない」と嘘をつきます。**
 * 未設定と `null` は同じ意味に揃えます（Rust 側は省略できる項目を出さないことがある）。
 */
function sameShape(one: Connection, other: Connection): boolean {
	const left = one as unknown as Record<string, unknown>;
	const right = other as unknown as Record<string, unknown>;
	const keys = new Set([...Object.keys(left), ...Object.keys(right)]);

	return [...keys].every((key) => JSON.stringify(left[key] ?? null) === JSON.stringify(right[key] ?? null));
}
