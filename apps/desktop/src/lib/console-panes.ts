/**
 * **端末を 2 枚に割る規則**（DESIGN.md「端末の割り方」・2026-10-06）。
 *
 * 人が決めた形 ——
 *
 * > 見た目はデスクトップみたいに**半々**とかタブ切り替えかなぁ
 * > VSCode もそんな感じだったような
 * > **番号とかタイトルとかいろとかできると尚良い** w
 *
 * **ここは「どの端末がどちらの面に出るか」だけを決めます。**
 * 描く所（xterm）とは分けてあります —— 規則だけなら、**画面を描かずに
 * 確かめられます。**端末の面は 2026-09-01 から 3 日間 1 バイトも映しておらず、
 * 型検査は 0 errors のままでした（Issue #10）。**描く所に規則を混ぜません。**
 *
 * **幅は扱いません。**`$lib/splitter.svelte` は掴んで動かす幅を覚える仕組みで、
 * 別の話です。DESIGN.md のとおり**等分から始めます**（掴む境目は、
 * 要ると分かってから足す・YAGNI）。
 */

/** 端末 1 枚。**実行体（`console_list`）が返すものと同じ形。** */
export interface ConsolePane {
	/** 番号。**使い回されません**（閉じた番号が別の端末を指すことはない）。 */
	readonly id: number;
	/** どの接続か。**識別子だけ**（ホスト名は入りません）。 */
	readonly connection: string;
	readonly holder: 'human' | 'ai';
}

/**
 * **同時に並べる面の数。**
 *
 * **2 なのは「半々」だから**です。3 分割は、狭い画面で読めません ——
 * 読めない面は「開いているのに見えない面」で、それは
 * 「人が常に見ている」（D29）を壊します。
 *
 * **実行体の上限（`PER_CONNECTION_LIMIT`）と揃えてあります。**
 * どちらかだけ上げると、**開けるのに映らない**か**押しても断られる**になります。
 */
export const PANES = 2;

export interface Layout {
	/** 並べて出す面。**番号の順**（並びが変わると左右を取り違えます）。 */
	readonly panes: readonly ConsolePane[];
	/** 出しきれなかったもの。**タブで切り替えます。** */
	readonly spare: readonly ConsolePane[];
	/** いま打鍵が行く面。**無いこともあります**（1 枚も開いていない）。 */
	readonly focused: number | null;
	/** もう 1 枚開けるか。**押しても断られる釦を出さないため。** */
	readonly canOpenAnother: boolean;
}

/**
 * いま見ている接続の端末を、面へ割り振る。
 *
 * - `all` … 開いている端末ぜんぶ（別の接続のものも混ざっています）
 * - `connection` … 人が見ている接続。`null` なら繋がっていない
 * - `chosen` … 人が選んだ番号。**閉じていたら忘れます**
 */
export function paneLayout(
	all: readonly ConsolePane[],
	connection: string | null,
	chosen: number | null
): Layout {
	if (connection === null) {
		return { panes: [], spare: [], focused: null, canOpenAnother: false };
	}

	// **宛先の分だけ。**別のサーバの面を混ぜると、人はどちらで打ったのか分かりません。
	// **番号の順に並べます** —— 並びが毎回変わると、左右を取り違えます。
	const ordered = all
		.filter((pane) => pane.connection === connection)
		.toSorted((a, b) => a.id - b.id);

	if (ordered.length === 0) {
		return { panes: [], spare: [], focused: null, canOpenAnother: true };
	}

	// **選んだものは必ず出す。**出ないなら、押せない釦と同じです。
	// 閉じた番号を選んだままにはしません（番号は使い回されないので、
	// 「消えた」以外の意味になりません）。
	const picked = ordered.find((pane) => pane.id === chosen) ?? ordered[0];
	const others = ordered.filter((pane) => pane.id !== picked.id);
	const shown = [picked, ...others.slice(0, PANES - 1)].toSorted((a, b) => a.id - b.id);
	const shownIds = new Set(shown.map((pane) => pane.id));

	return {
		panes: shown,
		spare: ordered.filter((pane) => !shownIds.has(pane.id)),
		focused: picked.id,
		canOpenAnother: ordered.length < PANES
	};
}
