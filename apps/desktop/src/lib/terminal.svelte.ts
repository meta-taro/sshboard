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

export function createTerminal(host: HTMLElement): Terminal {
	const terminal = new Terminal({
		convertEol: false,
		cursorBlink: false,
		disableStdin: true,
		fontSize: 12,
		fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
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
