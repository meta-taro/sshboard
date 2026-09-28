/**
 * AI が「こちらを見てください」と言ってきたとき、**従ってよいか**（D44 / Issue #15）。
 *
 * `focus_connection` は**操作の宛先**を変えるもので、**人が見ている画面は動きません。**
 * 画面を動かす口が MCP に無いため、AI は「端末タブを開いてください」と
 * 文章で頼むしかありませんでした。**それはこの製品が消したかったはずの往復**です。
 *
 * ここが決めるのは**切り替えるかどうか**だけ。**承認も実行も増えません。**
 */

/**
 * 画面のタブ。**MCP へ渡す値と同じ**にします（D44）。
 *
 * `terminal` ではなく `console`、`activity` / `log` ではなく `band` / `diag`。
 * **製品の中で呼び名を 1 つに**するためで、MCP の値は後から変えにくいためです。
 */
export const VIEWS = ['connections', 'files', 'console', 'band', 'diag'] as const;

export type View = (typeof VIEWS)[number];

export function isView(name: string): name is View {
	return (VIEWS as readonly string[]).includes(name);
}

/**
 * 人が自分で切り替えたあと、AI に触らせない時間。
 *
 * **短すぎると奪ったのと同じ**（選んだ端から戻される）。
 * **長すぎると口が無いのと同じ**（頼んでも効かない）。
 */
export const HUMAN_HOLDS_MS = 8000;

/** 従ってよいかを決める材料。**画面の状態だけ**で、AI 側の事情は見ません。 */
export interface ScreenState {
	/** 問い（パスフレーズ・端末の許可）が出ているか。 */
	readonly askOpen: boolean;
	/** 入力欄に焦点があるか。 */
	readonly focusInField: boolean;
	/** 人が自分でタブを切り替えてからの経過。 */
	readonly sinceHumanSwitchMs: number;
}

/**
 * **奪わない。**次のどれかに当たっていれば、頼まれても動きません。
 *
 * 1. **問いが出ている** —— 動かすと、人は何に答えたのか分からなくなります
 * 2. **入力欄に焦点がある** —— 動かすと、打っていたものが消えます
 * 3. **人が直前に自分で選んだ** —— 選んだ端から戻すのは、奪ったのと同じです
 */
export function canFollow(screen: ScreenState): boolean {
	if (screen.askOpen) return false;
	if (screen.focusInField) return false;
	return screen.sinceHumanSwitchMs >= HUMAN_HOLDS_MS;
}

/**
 * **人が問いに答えたあと、画面が自分で動く先**（実運用の指摘・2026-09-28）。
 *
 * 実機の言葉 ——
 *
 * > そもそも端末をにぎりたいとアラートがでて、**通った時点で、一緒にタブも
 * > 切り替わる実装にすべき**とつたえてください。
 *
 * 実際に困った所 —— 掲載停止の作業中、AI が端末を握って `php -l` や `rm` を
 * 打っていたのに、**人の画面は別のタブのまま**でした。
 * 「端末タブにきりかえてください」と言われて、初めて `show_view` を呼んでいます。
 *
 * **この製品の売りは「同じ画面を一緒に見る」ことです。**
 * **端末を許可したのに端末が見えていないのは、その一点が抜けています。**
 *
 * ## `canFollow` を通しません
 *
 * あれは **AI の頼み**を選り分けるものです。ここは違います ——
 * **「端末を使ってよいか」に「よい」と答えた人は、端末を見るつもりで答えています。**
 * **人が自分で押した結果**なので、奪ったことになりません。
 *
 * ## 断ったときは動かしません
 *
 * 見る理由がないからです。**断った人を端末へ連れて行くのは、押し売り**です。
 */
export function viewAfterAnswered(ask: 'console', allowed: boolean): View | null {
	return ask === 'console' && allowed ? 'console' : null;
}
