/**
 * 立ち上げ直したあと、**開いていた接続へ繋ぎ直すかを聞く**（UI 案・5 枚目）。
 *
 * ## なぜ要るか
 *
 * この製品の「再起動」は、他のアプリの再起動と重さが違います ——
 * **生きている SSH・開いている端末・転送中のファイル**が落ち、
 * MCP が共有している同じ 1 本も道連れになります（禁止事項 3）。
 *
 * それまで、立ち上げ直したあとは**何も起きませんでした。**
 * 人は「どこに繋いでいたか」を自分で思い出して、1 つずつ繋ぎ直していました。
 *
 * ## 何を控えるか
 *
 * **接続の識別子だけ**です。ホスト・利用者名・鍵・パスは入れません
 * （禁止事項 4 / product-baseline §14）。識別子は人が付けた名札で、
 * **それ自体は接続先を明かしません。**
 *
 * ## 勝手には繋ぎません
 *
 * **聞くだけ**です。立ち上げ直した瞬間に繋ぎ直すと、
 * **人が見ていない所で SSH が開きます**（禁止事項 3）。
 *
 * **AI の分も繋ぎ直しません。**AI のほうからもう一度呼ばれたときに繋がります
 * （案もそう書いています）。
 */

/** 控えの置き場。**`splitter.svelte.ts` と同じ作法**（前置きを付ける）。 */
const KEY = 'sshboard-reopen';

/**
 * 控えが古くなったら忘れる。
 *
 * **立ち上げ直しの直後だけ**聞きたいので、10 分を過ぎたものは出しません ——
 * 翌朝に「昨日の夜に開いていたものへ繋ぎ直しますか」と聞かれても、
 * 人には何の話か分かりません。
 */
export const STILL_FRESH_MS = 10 * 60 * 1000;

export type Reopen = {
	/** 控えた時刻（epoch ミリ秒）。 */
	at: number;
	/** 開いていた接続の識別子。**ホストも鍵も入っていません。** */
	ids: string[];
};

/**
 * 立ち上げ直す前に控える。
 *
 * **1 件も開いていなければ、何も書きません** ——
 * 空の控えを残すと、次の起動で「0 件へ繋ぎ直しますか」を出しかねません。
 */
export function remember(ids: readonly string[], now = Date.now()): void {
	try {
		if (ids.length === 0) {
			localStorage.removeItem(KEY);
			return;
		}
		localStorage.setItem(KEY, JSON.stringify({ at: now, ids: [...ids] }));
	} catch {
		// **握り潰してよい唯一の所。**控えられなくても、
		// 立ち上げ直し自体は止めません（聞けないだけ）。
	}
}

/**
 * 控えを読む。**読んだら消します**（一度しか聞かない）。
 *
 * 壊れていたら `null`。**信じられない控えで人に問わない**ためで、
 * 「0 件へ繋ぎ直しますか」や「`undefined` へ繋ぎ直しますか」は出しません。
 */
export function take(now = Date.now()): Reopen | null {
	let raw: string | null = null;
	try {
		raw = localStorage.getItem(KEY);
		localStorage.removeItem(KEY);
	} catch {
		return null;
	}
	if (!raw) return null;

	try {
		const parsed: unknown = JSON.parse(raw);
		if (typeof parsed !== 'object' || parsed === null) return null;
		const { at, ids } = parsed as { at?: unknown; ids?: unknown };
		if (typeof at !== 'number' || !Number.isFinite(at)) return null;
		if (!Array.isArray(ids)) return null;

		const named = ids.filter((id): id is string => typeof id === 'string' && id.trim() !== '');
		if (named.length === 0) return null;

		// **古い控えは出さない。**立ち上げ直しの直後だけ聞きます。
		// **未来の時刻も捨てます** —— 時計が戻った機械で、永久に残るのを防ぐため。
		const age = now - at;
		if (age < 0 || age > STILL_FRESH_MS) return null;

		return { at, ids: named };
	} catch {
		return null;
	}
}

/** 控えを捨てる（人が「繋ぎ直さない」と答えたとき）。 */
export function forget(): void {
	try {
		localStorage.removeItem(KEY);
	} catch {
		// 同上。
	}
}
