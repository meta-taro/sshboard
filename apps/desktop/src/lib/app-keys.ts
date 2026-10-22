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

/**
 * **キー操作の一覧**（UI 案・2026-10-21 のヘルプ）。
 *
 * ## なぜ表にするか
 *
 * `Ctrl Shift H`（画面を伏せる）を入れても、**どこにも書いていなければ
 * 無いのと同じ**です。Issue #9 で一度やりました ——
 * 「2 台目に繋げるのか」を探している人に、案内が見えていなかった。
 *
 * ## なぜ `appIntent` と同じ所に置くか
 *
 * **ヘルプが実物とずれるのが、一番たちの悪い壊れ方**です
 * （書いてあるのに効かない／効くのに書いていない）。
 * ここに置けば、**表から事象を組み立てて `appIntent` に通し、
 * 本当にその動作になるか**を機械が確かめられます（`app-keys.test.ts`）。
 */
export interface Shortcut {
	/** どの動作か。**`AppIntent` を全部網羅していることを試験が数えます。** */
	readonly intent: AppIntent;
	/** 押すキー（飾りを除いた本体）。 */
	readonly key: string;
	readonly shift?: boolean;
	/** `Ctrl`（mac では `⌘`）が要るか。`F1` のように要らないものもある。 */
	readonly held?: boolean;
	/** 画面に出す名前。**文言は既に在る鍵を使い回します**（言い方を増やさない）。 */
	readonly label: string;
}

export const APP_SHORTCUTS: readonly Shortcut[] = [
	{ intent: 'menu', key: ',', held: true, label: 'menu.view' },
	{ intent: 'help', key: 'F1', label: 'help.open' },
	{ intent: 'mask', key: 'h', held: true, shift: true, label: 'mask.hide' },
	{ intent: 'stopAi', key: '.', held: true, shift: true, label: 'ai.stop' },
	{ intent: 'newConnection', key: 'n', held: true, label: 'conn.new' },
	{ intent: 'view:1', key: '1', held: true, label: 'tab.connections' },
	{ intent: 'view:2', key: '2', held: true, label: 'tab.files' },
	{ intent: 'view:3', key: '3', held: true, label: 'tab.console' },
	{ intent: 'view:4', key: '4', held: true, label: 'tab.band' },
	{ intent: 'view:5', key: '5', held: true, label: 'tab.diag' }
];

/**
 * **他の担当が持っているキー。**
 *
 * ここでは**判定しません**（`appIntent` は `null` を返します）。
 * それでも人が見る表には要ります —— **誰が持っていようと、
 * 押す人には同じ 1 枚の表**です。
 */
export interface ForeignShortcut {
	readonly key: string;
	readonly shift?: boolean;
	/** `Ctrl`（mac では `⌘`）が要るか。 */
	readonly held?: boolean;
	/** `Alt`（mac では `⌥`）が要るか。 */
	readonly alt?: boolean;
	readonly label: string;
	/** 持ち主。**試験がそのファイルを読んで、いまもこのキーかを確かめます。** */
	readonly module: string;
}

export const OTHER_SHORTCUTS: readonly ForeignShortcut[] = [
	{ key: 'c', held: true, shift: true, label: 'menu.copy', module: 'terminal-clipboard' },
	{ key: 'v', held: true, shift: true, label: 'menu.paste', module: 'terminal-clipboard' },
	{ key: 'f', held: true, shift: true, label: 'search.label', module: 'terminal-search' },
	{ key: 'F5', label: 'files.refresh', module: 'FileBrowser' },
	{ key: 'ArrowUp', alt: true, label: 'files.up', module: 'FileBrowser' },
	{ key: 'ArrowLeft', alt: true, label: 'files.back', module: 'FileBrowser' },
	{ key: 'ArrowRight', alt: true, label: 'files.forward', module: 'FileBrowser' }
];

/** 人が読む形にする（`⌘ Shift H` / `Ctrl Shift H`）。 */
export function spellShortcut(
	shortcut: { key: string; shift?: boolean; held?: boolean; alt?: boolean },
	platform: AppPlatform
): string {
	const parts: string[] = [];
	if (shortcut.held) parts.push(platform === 'mac' ? '⌘' : 'Ctrl');
	if (shortcut.alt) parts.push(platform === 'mac' ? '⌥' : 'Alt');
	if (shortcut.shift) parts.push('Shift');
	// 1 文字のキーは大文字で出す（`h` ではなく `H`）。**押すのは同じキー。**
	parts.push(shortcut.key.length === 1 ? shortcut.key.toUpperCase() : shortcut.key);
	return parts.join(' ');
}
