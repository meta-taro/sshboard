/**
 * 変更内容を札へ収める（D34 の追記 3）。
 *
 * これまで `found.body` を受け取っていながら、**画面のどこにも出していませんでした。**
 * D34 の「黙って入れ替えない・押すのは人」は、**押す材料を渡して初めて成立します。**
 * 版番号だけを見せて「はい」を押させるのは、判断ではありません。
 *
 * **札の中にスクロール箱を作りません。**先頭だけを出し、残りは件数で示して、
 * 押したときにその場で伸ばします。比べた 2 つのアプリはどちらも箱の中で
 * 文の途中が切れていて、**切れていること自体に気づけない**形でした。
 *
 * 本文は**配布元から来る任意の文字列**です。長さも行数もここで切ります
 * （画面側は `{@html}` を使いません。Svelte の既定のままで差し込まれません）。
 */

/** 1 行の長さの上限（文字数）。これを超えたら切って `…` を付けます。 */
export const MAX_NOTE_LINE = 120;

/** 全文の行数の上限。**配布元が何行送ってきても、画面が伸び続けない。** */
export const MAX_NOTE_LINES = 40;

/** 既定で札に出す行数。 */
const DEFAULT_HEAD = 3;

/** 行頭の飾り。`## ` `- ` `* ` `+ ` `1. ` `・`。**文中のハイフンは残します。** */
const LEADING_MARK = /^\s*(?:#{1,6}\s+|[-*+]\s+|\d+[.)]\s+|・\s*)/;

export type NotesSummary = {
	/** 札にそのまま出す行。 */
	head: string[];
	/** まだ出していない行の数。**0 なら「全文」を出しません。** */
	rest: number;
	/** 開いたときに出す全文（整形済み・上限つき）。 */
	lines: string[];
};

/** 1 行を整える。飾りを落とし、長すぎれば切る。 */
function tidy(line: string): string {
	const bare = line.replace(LEADING_MARK, '').trim();
	if (bare.length <= MAX_NOTE_LINE) return bare;
	return bare.slice(0, MAX_NOTE_LINE) + '…';
}

/**
 * 畳む。**空なら空を返します**（「変更内容」の見出しだけが出るのを避けるため）。
 */
export function summarise(notes: string, max: number = DEFAULT_HEAD): NotesSummary {
	const lines = notes
		.split(/\r?\n/)
		.map(tidy)
		.filter((line) => line !== '')
		.slice(0, MAX_NOTE_LINES);

	const head = lines.slice(0, Math.max(0, max));
	return { head, rest: lines.length - head.length, lines };
}
