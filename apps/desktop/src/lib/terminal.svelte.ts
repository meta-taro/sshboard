/**
 * xterm.js の生成と、そこへ生の出力を書き込む口。
 *
 * **ANSI の解釈を自前で書かない**（D7）。ここがやるのは、
 * Rust から届いたバイト列をそのまま渡すことだけ。
 */
import { Terminal } from '@xterm/xterm';

/** Phase 0 の確認用。配色・字送りは DESIGN.md で人が決める。 */
const PLACEHOLDER_THEME = {
	background: '#16181d',
	foreground: '#d7dae0'
};

/**
 * 端末を作る。`fontSize` は**画面側の文字サイズと揃えた px**（`text-size.svelte.ts`）。
 *
 * xterm.js は自前で描画するため `rem` が効かない。**px を渡すしかない。**
 */
export function createTerminal(host: HTMLElement, fontSize = 12): Terminal {
	const terminal = new Terminal({
		convertEol: false,
		cursorBlink: false,
		disableStdin: true,
		fontSize,
		// 日本語のグリフを持つ等幅フォントを候補に入れる。
		// **入れないと、xterm.js が 2 桁分を確保したところへ 1 桁幅の字が描かれ、
		// 文字と文字の間が空く。**対象が国内サーバーなので、ここは実務で必ず踏む。
		// **どのフォントを既定にするかは人が決める領域**（DESIGN.md）。ここは仮置き。
		fontFamily:
			'ui-monospace, SFMono-Regular, Menlo, Consolas, "Osaka-Mono", "BIZ UDGothic", "MS Gothic", "Noto Sans Mono CJK JP", monospace',
		scrollback: 5000,
		theme: PLACEHOLDER_THEME
	});
	terminal.open(host);
	return terminal;
}

/**
 * Tauri から届く生のチャンクを書き込む。
 *
 * イベントの payload は数値の配列で来る。**そのままバイト列として渡す。**
 * 文字列へ変換すると、UTF-8 の途中で切れたチャンクが壊れる。
 */
export function writeChunk(terminal: Terminal, payload: unknown): void {
	if (!Array.isArray(payload)) return;
	terminal.write(Uint8Array.from(payload as number[]));
}
