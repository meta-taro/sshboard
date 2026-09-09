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
