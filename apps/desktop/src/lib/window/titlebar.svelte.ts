/**
 * 自前のタイトルバーを動かす（D17）。
 *
 * **`decorations: false` にしたので、この帯が OS のタイトルバーそのものです。**
 * 標準のタイトルバーは配色がテーマに追従せず、暗いアプリの上に明るい帯が乗ります
 * （Windows 実機で実際にそうなった）。dbboard / md-business と揃える意味もあります。
 *
 * **Rust 側は 1 行も要りません。**`@tauri-apps/api/window` の
 * `minimize` / `toggleMaximize` / `close` を叩くだけです。
 *
 * **Tauri の外（素の `vite dev`）でも落ちません。**帯は描かれ、ボタンだけが効かなくなります。
 * 画面を開発するのに窓を用意させない方が回ります。
 */

let maximized = $state(false);

function inTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** **遅延読み込み。**ブラウザで走らせたときに Tauri の API を触らないため。 */
async function appWindow() {
	const { getCurrentWindow } = await import('@tauri-apps/api/window');
	return getCurrentWindow();
}

export const titlebar = {
	/** いま最大化しているか。 */
	get isMaximized(): boolean {
		return maximized;
	},

	/** 最大化ボタンの字。**戻す側と広げる側で見た目を変える。** */
	get maximizeGlyph(): string {
		return maximized ? '❐' : '▢';
	},

	/** 起動時に今の状態を取り込み、以後の変化を追う。 */
	async init(): Promise<void> {
		if (!inTauri()) return;
		try {
			const w = await appWindow();
			maximized = await w.isMaximized();
			await w.onResized(async () => {
				try {
					maximized = await w.isMaximized();
				} catch {
					// 権限が無ければ追うのをやめる。**表示が古くなるだけで、操作は死なない。**
				}
			});
		} catch {
			// Tauri の外。帯は出て、ボタンだけが効かない。
		}
	},

	async minimize(): Promise<void> {
		if (!inTauri()) return;
		try {
			await (await appWindow()).minimize();
		} catch {
			// 窓の操作が拒まれた。**画面を壊さない。**
		}
	},

	async toggleMaximize(): Promise<void> {
		if (!inTauri()) return;
		try {
			const w = await appWindow();
			await w.toggleMaximize();
			maximized = await w.isMaximized();
		} catch {
			// 同上
		}
	},

	async close(): Promise<void> {
		if (!inTauri()) return;
		try {
			await (await appWindow()).close();
		} catch {
			// 同上
		}
	}
};
