/**
 * xterm.js の生成と、そこへ生の出力を書き込む口。
 *
 * **ANSI の解釈を自前で書かない**（D7）。ここがやるのは、
 * Rust から届いたバイト列をそのまま渡すことだけ。
 */
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Terminal } from '@xterm/xterm';

import { createSearch, type TerminalSearch } from './terminal-search';

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
export function createTerminal(host: HTMLElement, fontSize = 12, typable = false): Terminal {
	const terminal = new Terminal({
		convertEol: false,
		// **打てる面では点滅させる。**どこに入るのかが見えないと打てない。
		cursorBlink: typable,
		// 出力を見るだけの面では、打てないことを**構造で**示す（D29 のロック）。
		disableStdin: !typable,
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
 * 端末を入れ物の大きさに合わせ、**その後もついていくようにする。**
 *
 * これが無いと xterm.js は **80×24 のまま**で、窓を広げても増えません
 * （実際にそうなっていました・2026-09-01）。桁数が合わないと、
 * サーバー側が折り返しの位置を誤り、**表示が崩れます。**
 *
 * 大きさが変わると `onResize` が出るので、そこから `console_resize` が
 * サーバーへ伝わります。**配線はあったのに、変わる元が無かった。**
 */
export function attachFit(terminal: Terminal, host: HTMLElement): () => void {
	const addon = new FitAddon();
	terminal.loadAddon(addon);

	const refit = () => {
		try {
			addon.fit();
		} catch {
			// **要素がまだ 0px のときに投げます。**タブを切り替えた直後がこれ。
			// 次の通知で正しい大きさが来るので、ここは待つのが正しい。
			// 握り潰しに見えますが、**捨てているのは「まだ測れない」という事実だけ**です。
		}
	};

	refit();
	const observer = new ResizeObserver(refit);
	observer.observe(host);
	return () => observer.disconnect();
}

/**
 * 端末に検索を付ける。**探し方も強調も xterm 公式の addon 任せ**（D7）。
 */
export function attachSearch(
	terminal: Terminal,
	onError: (error: unknown) => void
): TerminalSearch {
	const addon = new SearchAddon();
	terminal.loadAddon(addon);
	return createSearch(addon, onError);
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
