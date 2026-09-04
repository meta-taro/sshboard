/**
 * 再起動で何が切れるかを数える（D34 の追記 3）。
 *
 * **この製品の「再起動」は、他のアプリの再起動と重さが違います。**
 * エディタなら未保存の文書、DB クライアントなら実行中のクエリで済みますが、
 * sshboard は **生きている SSH・開いている端末・転送中のファイル**を落とし、
 * さらに **MCP が共有している同じ 1 本**（CLAUDE.md 禁止事項 3）も道連れにします。
 *
 * **押す前に、何が切れるかを名乗ります。**止めはしません。止めるのは人の判断で、
 * こちらの仕事は「知らないまま押させない」ことです。
 */

/** 切れるもの。**0 と「端末だけ」を区別するため、両方を持ちます。** */
export type RestartCost = {
	sessions: number;
	terminal: boolean;
};

/** 画面が出す文の鍵。**足し算で 1 文に押し込むと、訳が不自然になる。** */
export type CutMessageKey = 'update.cuts' | 'update.cuts.both' | 'update.cuts.terminal';

/**
 * 数える。**切れるものが無ければ `null`。**
 *
 * 数えられない値（負・小数・NaN）は 0 として扱います。
 * **信じられない数を人へ見せない**ためで、「-1 本が切れます」は出しません。
 */
export function cutSummary(input: { sessions: number; terminalOpen: boolean }): RestartCost | null {
	const counted = Number.isInteger(input.sessions) && input.sessions > 0 ? input.sessions : 0;
	const terminal = input.terminalOpen === true;

	if (counted === 0 && !terminal) return null;
	return { sessions: counted, terminal };
}

/** どの文を出すか。 */
export function cutMessageKey(cost: RestartCost): CutMessageKey {
	if (cost.sessions === 0) return 'update.cuts.terminal';
	return cost.terminal ? 'update.cuts.both' : 'update.cuts';
}
