/**
 * 端末の中を検索する。
 *
 * ## 触ってはいけない組み合わせ
 *
 * | 組み合わせ | 端末での意味 | ここでの扱い |
 * |---|---|---|
 * | `Ctrl+F` | **1 文字進む**（emacs 流の割り当て） | **横取りしない** |
 *
 * `Ctrl+F` を検索に取ると、**行の中を動けなくなります。**
 * だから Windows / Linux 側は `Ctrl+Shift+F`。`⌘F` はメニューに無いので使えます。
 *
 * 検索そのものは xterm 公式の `@xterm/addon-search` がやります。
 * **正規表現も強調表示も自前で書きません**（D7 と同じ考え方）。
 */
import type { TerminalPlatform } from './terminal-clipboard';

/** 判定に要る分だけ。 */
export interface SearchShortcutEvent {
	readonly type: string;
	readonly key: string;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
}

/** 差し替えられる検索 addon。**本物の `SearchAddon` もこの形を満たします。** */
export interface SearchAddonLike {
	findNext(term: string): boolean;
	findPrevious(term: string): boolean;
	clearDecorations(): void;
}

/** 画面から呼ぶ口。 */
export interface TerminalSearch {
	next(term: string): boolean;
	previous(term: string): boolean;
	/** 閉じる。**強調を消します。**残すと、いま何を見ているのか分からなくなる。 */
	close(): void;
}

/**
 * 検索の組み合わせか。**素の `Ctrl+F` は常に `false`。**
 */
export function isFindShortcut(event: SearchShortcutEvent, platform: TerminalPlatform): boolean {
	if (event.type !== 'keydown') return false;
	if (event.key.toLowerCase() !== 'f') return false;

	if (platform === 'mac') {
		return event.metaKey && !event.ctrlKey;
	}
	return event.ctrlKey && event.shiftKey && !event.metaKey;
}

/**
 * 検索の口を作る。
 *
 * `onError` は**握り潰さないため**の口です。addon は描画が整う前に呼ぶと
 * 投げることがあり、黙って `false` を返すと「押したのに何も起きない」になります。
 */
export function createSearch(
	addon: SearchAddonLike,
	onError: (error: unknown) => void = () => {}
): TerminalSearch {
	// 空で検索すると**端末じゅうが光ります。**押し間違いで普通に起きる。
	const usable = (term: string) => term.trim() !== '';

	const run = (find: (term: string) => boolean, term: string): boolean => {
		if (!usable(term)) return false;
		try {
			return find(term);
		} catch (error: unknown) {
			onError(error);
			return false;
		}
	};

	return {
		next: (term: string) => run((value) => addon.findNext(value), term),
		previous: (term: string) => run((value) => addon.findPrevious(value), term),
		close: () => {
			try {
				addon.clearDecorations();
			} catch (error: unknown) {
				onError(error);
			}
		}
	};
}
