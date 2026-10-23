/**
 * **不具合の報告に貼る 4 行**（案・4 枚目「版情報を写す」）。
 *
 * ## なぜ要るか
 *
 * 報告に最初に要るのは版です（`about.alpha` にもそう書いてあります）。
 * **それまで、画面には出ているのに写せませんでした** ——
 * 人は目で読んで手で打ち直すことになります。打ち直せば間違います。
 *
 * ## 利用者名を入れません
 *
 * **案は「設定の置き場所」を 4 行に含めています。**そのまま写すと、
 * macOS と Linux では `/Users/<名前>/…` が入ります。
 * **これは public リポジトリの Issue へ貼られる文です**
 * （product-baseline §25 / `oss-privacy-check` が検出します）。
 *
 * **家の場所は `~` に置き換えます。**どの OS のどこに在るかは伝わり、
 * 誰の機械かは伝わりません。置き換えは実行体側でも行いますが
 * （受け取らないものは漏らせない）、**ここでも落とします** ——
 * 二重にするのは、片方が壊れたときに残る方が要るからです。
 */

/** 実行体から受け取る素材。 */
export type ReportFacts = {
	/** アプリの版。取れなければ `null`（**嘘の版を出しません**）。 */
	version: string | null;
	/** `macos · aarch64` のような形。 */
	platform: string | null;
	/** 設定の置き場所。**家の部分は `~`** に置き換わっています。 */
	configDir: string | null;
	/** MCP が待ち受けているか。 */
	mcpRunning: boolean;
};

/** 入っていない所に書く印。**空欄を `N/A` で埋めません**（§19）。 */
const UNKNOWN = '不明';

/**
 * 家の場所を `~` にする。
 *
 * **`/Users/<名前>` と `/home/<名前>` と `C:\\Users\\<名前>` の 3 つ**を見ます。
 * 見つからなければそのまま返します（**壊して短くするより、そのまま出す**）。
 */
export function withoutHome(path: string): string {
	return path
		.replace(/^\/Users\/[^/]+/, '~')
		.replace(/^\/home\/[^/]+/, '~')
		.replace(/^[A-Za-z]:\\Users\\[^\\]+/, '~');
}

/**
 * 貼る文を組む。**4 行、ちょうど。**
 *
 * 行を増やすと、人は貼る前に読み直して削ります。**削られたら意味がありません。**
 */
export function reportText(facts: ReportFacts): string {
	return [
		`sshboard ${facts.version ?? UNKNOWN}`,
		`環境: ${facts.platform ?? UNKNOWN}`,
		`MCP: ${facts.mcpRunning ? '待ち受け中' : '止まっています'}`,
		`設定: ${facts.configDir ? withoutHome(facts.configDir) : UNKNOWN}`
	].join('\n');
}
