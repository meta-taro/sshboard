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

/**
 * 帯の絞り込み（UI 案・2026-10-21）。
 *
 * 案は帯の肩に「全部 / AI だけ / 人だけ」を置きました。
 *
 * **なぜ要るか** —— 帯は人と AI の操作が時間順に 1 本で並びます。
 * それが帯の値打ちですが、**「さっき AI は何をしたのか」だけを追いたい**
 * ときに、自分の打鍵が間に挟まって読めません。逆もあります。
 *
 * **混ぜたものを分けて見せるだけで、混ぜるのをやめません** ——
 * 既定は「全部」です。1 本に並んでいることが、この道具の要です。
 */
export type BandWho = 'all' | 'ai' | 'human';

/** Rust 側が行頭に付ける札。**ここを変えると絞りが黙って効かなくなる。** */
export const AI_TAG = '[AI]';
export const HUMAN_TAG = '[Human]';

/**
 * 誰の行かで絞る。**元の配列は書き換えません。**
 *
 * 札が `[AI]` でも `[Human]` でもない行（将来足したもの）は、
 * **「全部」のときだけ出します** —— 知らない札を人だと決めつけると、
 * **AI の操作を人のものとして見せる**ことになります。そちらが危ない方です。
 */
export function filterBand(lines: readonly BandLine[], who: BandWho): BandLine[] {
	if (who === 'all') return [...lines];
	const want = who === 'ai' ? AI_TAG : HUMAN_TAG;
	return lines.filter((line) => line.tag === want);
}
