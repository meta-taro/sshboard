/** 帯の 1 行。Rust 側の `BandLinePayload` と対になる。 */
export type BandLine = {
	seq: number;
	/** `[AI]` / `[Human]` */
	tag: string;
	text: string;
	/** 行頭を揃えた表示用の 1 行 */
	rendered: string;
};

/** 画面に残す行数の上限。`tail -f` を流す Phase 0-5 で青天井にしないため。 */
export const MAX_VISIBLE_LINES = 2000;

/** 行を足した新しい配列を返す。**既存の配列を書き換えない。** */
export function appendLine(lines: readonly BandLine[], line: BandLine): BandLine[] {
	const next = [...lines, line];
	return next.length > MAX_VISIBLE_LINES ? next.slice(next.length - MAX_VISIBLE_LINES) : next;
}
