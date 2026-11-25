/**
 * **過去の帯**（Phase 2「監査ログの永続化」・2026-11-25）。
 *
 * 帯は画面の中にしか無く、アプリを閉じると消えていました。実行体が帯を
 * この PC の中へ 1 日 1 本で控え（14 日で消します）、ここで読み戻します。
 *
 * 控えに入るのは**帯に出た行だけ**です（誰が・何を・どの接続のどのパス）。
 * パスワードと、コマンドの出力は入りません。
 */

/** 控えの 1 件。Rust 側の `JournalEntry` と対になる。 */
export type JournalEntry = {
	/** いつ（UNIX 時刻・ミリ秒）。 */
	atMs: number;
	/** 帯の中での通し番号。アプリを立ち上げ直すと 0 から数え直します。 */
	seq: number;
	/** `[AI]` / `[Human]` */
	tag: string;
	text: string;
};

/** 控えのある日と置き場所。Rust 側の `JournalOverview` と対になる。 */
export type JournalOverview = {
	dir: string;
	/** `YYYY-MM-DD`（UTC）。新しい順。 */
	days: string[];
	keepDays: number;
};

/** `[Human]` の 7 文字に揃える。**今の帯と同じ幅**（Rust の `TAG_WIDTH`）。 */
const TAG_WIDTH = 7;

const two = (value: number): string => String(value).padStart(2, '0');

/** 1 件を、手元の時計の時刻 ＋ 今の帯と同じ形の 1 行にする。 */
export function journalLine(entry: JournalEntry): string {
	const at = new Date(entry.atMs);
	const clock = `${two(at.getHours())}:${two(at.getMinutes())}:${two(at.getSeconds())}`;
	return `${clock} ${entry.tag.padEnd(TAG_WIDTH)} ${entry.text}`;
}
