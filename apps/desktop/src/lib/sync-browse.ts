/**
 * 左右のペインを連れて歩く（同期移動）。
 *
 * **絶対パスを合わせません。**合わせると、手元が `C:\Users\me` で
 * 相手が `/srv/app` のときに破綻します。**同じ「動き」をもう片方でもする**
 * 形にしています（WinSCP と同じ考え方）。
 *
 * 左で `logs` へ入ったら右も `logs` へ。左で上がったら右も上がる。
 * **パスを直接打った動きは伝えません** — 絶対パスは相手にとって意味がないためです。
 */

/** 片方で起きた動き。 */
export type Move =
	/** 名前の付いたフォルダへ入った。 */
	| { kind: 'into'; name: string }
	/** 1 つ上がった。 */
	| { kind: 'up' }
	/** パスを直接打った・履歴で飛んだ。**伝えません。** */
	| { kind: 'jump' };

/**
 * もう片方が行くべき場所を返す。**伝えないときは `null`。**
 *
 * `separator` は相手側の区切り（`/` か `\`）。
 */
export function mirrorMove(theirPath: string, move: Move, separator: string): string | null {
	if (move.kind === 'jump') return null;

	if (move.kind === 'into') {
		const base = theirPath.endsWith(separator) ? theirPath.slice(0, -1) : theirPath;
		return `${base}${separator}${move.name}`;
	}

	// 上がる。**根より上へは行きません。**
	const trimmed = theirPath.endsWith(separator) && theirPath.length > 1
		? theirPath.slice(0, -1)
		: theirPath;
	const cut = trimmed.lastIndexOf(separator);
	if (cut < 0) return theirPath;
	// `/logs` → `/`、`C:\work` → `C:\`
	const parent = trimmed.slice(0, cut);
	if (parent === '' ) return separator;
	if (parent.endsWith(':')) return `${parent}${separator}`;
	return parent;
}
