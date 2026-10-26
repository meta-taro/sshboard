/**
 * 診断の 1 件。Rust 側の `sshboard_diag::Event` と対。
 *
 * **なぜここに出したか** —— `+page.svelte` の中にインラインで書いてあり、
 * **診断の面を部品に出すと、型が 2 か所になる**所でした。
 * 2 か所に書いた型は、**片方だけ直る日**が来ます（D39）——
 * しかもここは Rust から降ってくる形なので、**ずれても型検査は通ります。**
 */
export type DiagEvent = {
	seq: number;
	/** 起動からの経過（ミリ秒）。**画面では秒に直して出します。日付ではありません。** */
	atMs: number;
	level: 'info' | 'warn' | 'error';
	/** どの段か（`Auth` / `Connect` など）。**Rust 側の `Stage` の名前がそのまま来ます。** */
	stage: string;
	/** どの接続の話か。**無い段もあります**（起動そのものの診断など）。 */
	connection?: string;
	message: string;
	/** **どうすれば通るか。**握り潰さないための一言（無いこともあります）。 */
	hint?: string;
};

/** 診断の「すべて」のタブ。接続の識別子と取り違えない印。 */
export const ALL = Symbol('all');

/** 診断のタブで選べるもの。「すべて」か、接続の識別子。 */
export type DiagPick = typeof ALL | string;

/**
 * **行のある接続の識別子**（オーナー・2026-10-26 ——「サーバごっちゃで読めない」）。
 *
 * **名前の順に固定します。**並びが毎回変わると、タブを押し間違えます。
 */
export function diagConnections(found: readonly DiagEvent[]): string[] {
	const ids = new Set<string>();
	for (const event of found) {
		if (event.connection) ids.add(event.connection);
	}
	return [...ids].toSorted();
}

/**
 * **選んだタブの行。**「すべて」は元の並びのまま全部。
 *
 * 接続の無い行（提案を押し出した など）は、**どの接続の話でもない**ので
 * 接続のタブには入れません（「すべて」にだけ出ます）。
 */
export function diagFor(found: readonly DiagEvent[], pick: DiagPick): DiagEvent[] {
	if (pick === ALL) return [...found];
	return found.filter((event) => event.connection === pick);
}
