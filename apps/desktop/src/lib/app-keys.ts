/**
 * 画面ぜんたいのキー操作（UI 案・2026-10-21）。
 *
 * 案はヘルプの画面に 6 つ並べました ——
 * `Ctrl ,`（メニュー）／`F1`（ヘルプ）／`Ctrl 1 / 2`（画面の切り替え）／
 * `Ctrl Shift H`（画面を伏せる）／`Ctrl Shift .`（AI を止める）／
 * `Ctrl N`（接続を足す）。
 *
 * ## なぜ別の部品にするか
 *
 * **取り合いが、押すまで分かりません。**この製品には既にキー操作の担当が
 * 3 つ在ります —— `edit-keys.ts`（入力欄の切貼り）、
 * `terminal-clipboard.ts`（`Ctrl Shift C` / `V`）、
 * `terminal-search.ts`（`Ctrl Shift F`）。
 *
 * ここで同じ組を取ると、**端末の上でコピーした瞬間に画面が伏せられます。**
 * 型検査も試験も通り、**押すまで分かりません。**
 * だから判定だけを切り出して、ぶつかっていないことを数えます。
 */

/** 何をしたいか。**該当しなければ null。** */
export type AppIntent =
	| 'menu'
	| 'help'
	| 'mask'
	| 'stopAi'
	| 'newConnection'
	/** 画面の切り替え。番号は 1 から。 */
	| `view:${1 | 2 | 3 | 4 | 5}`;

/** 判定に要るものだけ。**`KeyboardEvent` を丸ごと要求しない**（テストが重くなる）。 */
export interface AppKeyEvent {
	readonly key: string;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
	readonly altKey: boolean;
	/** 入力欄の中で押されたか。 */
	readonly inField: boolean;
}

export type AppPlatform = 'mac' | 'other';

/**
 * 画面の数。**案は 3 つ（端末 / ファイル / 接続）ですが、実物は 5 つです**
 * （＋帯・診断）。**案に無いものを消すのは再現ではない**ので 5 つまで受けます。
 */
const VIEWS = 5;

/** キーと OS から、したいことを決める。 */
export function appIntent(event: AppKeyEvent, platform: AppPlatform): AppIntent | null {
	// **入力欄の中では何もしない。**`F1` は押されますし、
	// 入力欄で画面が切り替わると**打っていたものが消えます。**
	if (event.inField) return null;

	// `Alt` はファイルの面の移動（`Alt + ←` / `Alt + ↑`）。**取り合いにしない。**
	if (event.altKey) return null;

	// `F1` だけは飾りを付けません。**OS を問わず同じキー**です。
	if (event.key === 'F1') {
		return event.ctrlKey || event.metaKey || event.shiftKey ? null : 'help';
	}

	// mac は Command、それ以外は Ctrl。**両方受けると、mac で端末の慣習と食い違います。**
	const held =
		platform === 'mac' ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
	if (!held) return null;

	const key = event.key.toLowerCase();

	if (event.shiftKey) {
		// **`c` / `v` / `f` はここに来ません。**端末の担当です
		// （`terminal-clipboard.ts` / `terminal-search.ts`）。
		if (key === 'h') return 'mask';
		if (key === '.' || key === '>') return 'stopAi';
		return null;
	}

	if (key === ',') return 'menu';
	if (key === 'n') return 'newConnection';

	// 画面の切り替え。**`+` / `-` / `0` は文字サイズなので取らない。**
	const number = Number(key);
	if (Number.isInteger(number) && number >= 1 && number <= VIEWS) {
		return `view:${number as 1 | 2 | 3 | 4 | 5}`;
	}

	return null;
}
