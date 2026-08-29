/**
 * 文字サイズ — 小 / 標準 / 大 / 特大。**既定は標準。**
 *
 * この製品は**画面いっぱいに小さい字が並ぶ**道具です。ファイル名・パス・ログ・
 * 指紋。読めなければ何も始まりません。**読める大きさは人によって違います。**
 *
 * 実装は `theme.svelte.ts` と同じ流儀です（`data-*` 属性 ＋ localStorage）。
 * 中身は `html` の `font-size` を変えるだけで、**画面側の CSS はすべて `rem`** なので
 * 余白や行間も一緒に伸びます。px で書いた場所があると、そこだけ伸びずに崩れます。
 *
 * **最初の描画より前に当てないと、一瞬だけ違う大きさが見えます**
 * （`app.html` で当てています）。
 */

export type TextSize = 'small' | 'normal' | 'large' | 'xlarge';

/** `app.html` と同じ鍵。**片方だけ変えると、最初の一瞬だけ別の大きさになる。** */
export const TEXT_SIZE_STORAGE_KEY = 'sshboard-text-size';

/** 小さい順。**この並びが「1 段上げる / 下げる」の順序です。** */
export const TEXT_SIZES: readonly TextSize[] = ['small', 'normal', 'large', 'xlarge'];

/**
 * 端末（xterm.js）の字の大きさ。**px で指定するしかないので、ここに表を持ちます。**
 *
 * xterm.js は自前で描画するため `rem` が効きません。**画面と端末で字の大きさが
 * 揃っていないと、同じ 1 つの道具に見えません。**
 */
const TERMINAL_PX: Record<TextSize, number> = {
	small: 11,
	normal: 12,
	large: 14,
	xlarge: 16
};

export function isTextSize(value: string | null): value is TextSize {
	return value === 'small' || value === 'normal' || value === 'large' || value === 'xlarge';
}

class TextSizeController {
	/** 人が選んだもの。 */
	mode = $state<TextSize>('normal');

	/** 端末へ渡す px。**画面側と揃えるため。** */
	get terminalPx(): number {
		return TERMINAL_PX[this.mode];
	}

	/** これ以上大きく / 小さくできないか。**押せないボタンを押させないため。** */
	get atLargest(): boolean {
		return this.mode === TEXT_SIZES[TEXT_SIZES.length - 1];
	}

	get atSmallest(): boolean {
		return this.mode === TEXT_SIZES[0];
	}

	init(): void {
		if (typeof window === 'undefined') return;

		const stored = localStorage.getItem(TEXT_SIZE_STORAGE_KEY);
		if (isTextSize(stored)) this.mode = stored;
		this.#apply();
	}

	set(mode: TextSize): void {
		this.mode = mode;
		if (typeof window !== 'undefined') {
			localStorage.setItem(TEXT_SIZE_STORAGE_KEY, mode);
		}
		this.#apply();
	}

	/** 1 段上げる / 下げる。**端で止まる**（一周させると行き過ぎに気づけない）。 */
	step(delta: 1 | -1): void {
		const at = TEXT_SIZES.indexOf(this.mode);
		const next = TEXT_SIZES[Math.min(TEXT_SIZES.length - 1, Math.max(0, at + delta))];
		if (next !== this.mode) this.set(next);
	}

	#apply(): void {
		if (typeof document === 'undefined') return;
		// **標準のときは属性を付けない。**付けないのが既定、が読みやすい。
		if (this.mode === 'normal') {
			document.documentElement.removeAttribute('data-text-size');
		} else {
			document.documentElement.setAttribute('data-text-size', this.mode);
		}
	}
}

export const textSize = new TextSizeController();
