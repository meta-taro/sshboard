/**
 * テーマ — 自動 / 明るい / 暗い。**既定は自動。**
 *
 * dbboard の `theme.svelte.ts` からの移植（D5）。
 *
 * 「自動」は OS に従うという意味で、`data-theme` を**付けません。**
 * `tokens.css` のメディアクエリが配色を決めます。
 * 明示的に選んだときだけ `data-theme` を付け、**両方向に**上書きします。
 *
 * 選択は localStorage に残します。**最初の描画より前に当てないと、
 * 一瞬だけ違うテーマが見えます**（`app.html` で当てています）。
 */

export type ThemeMode = 'auto' | 'light' | 'dark';

/** `app.html` と同じ鍵。**片方だけ変えると、最初の一瞬だけ別のテーマになる。** */
export const THEME_STORAGE_KEY = 'sshboard-theme';

export function isThemeMode(value: string | null): value is ThemeMode {
	return value === 'auto' || value === 'light' || value === 'dark';
}

class ThemeController {
	/** 人が選んだもの。`auto` は OS に従う。 */
	mode = $state<ThemeMode>('auto');

	/** 実際に出ている配色。`auto` を OS で解決したもの。 */
	resolved = $state<'light' | 'dark'>('dark');

	#media: MediaQueryList | null = null;

	init(): void {
		if (typeof window === 'undefined') return;

		const stored = localStorage.getItem(THEME_STORAGE_KEY);
		if (isThemeMode(stored)) this.mode = stored;

		this.#media = window.matchMedia('(prefers-color-scheme: dark)');
		// 自動のときは、OS 側の切り替えでその場で塗り替わる必要がある。
		this.#media.addEventListener('change', () => this.#apply());

		this.#apply();
	}

	set(mode: ThemeMode): void {
		this.mode = mode;
		if (typeof window !== 'undefined') {
			localStorage.setItem(THEME_STORAGE_KEY, mode);
		}
		this.#apply();
	}

	#apply(): void {
		if (typeof document === 'undefined') return;

		const osDark = this.#media?.matches ?? false;
		this.resolved = this.mode === 'auto' ? (osDark ? 'dark' : 'light') : this.mode;

		const root = document.documentElement;
		if (this.mode === 'auto') {
			// メディアクエリへ返す。
			root.removeAttribute('data-theme');
		} else {
			root.setAttribute('data-theme', this.mode);
		}

		// **`app.html` が最初の描画前に入れたインラインの `colorScheme` を外す。**
		//
		// 残したままだと、そこが CSS より強いので**切り替えても付いてこない**
		// （暗い → 明るいにしてもスクロールバーが暗いまま）。
		// 以後は `tokens.css` の 3 か所が決めます。
		root.style.removeProperty('color-scheme');
	}
}

export const theme = new ThemeController();
