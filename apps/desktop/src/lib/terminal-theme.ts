/**
 * 端末の配色を、**1 か所（`tokens.css`）から**組む。
 *
 * それまでは `terminal.svelte.ts` に仮置きの 2 色が直書きされており、
 * `tokens.css` の `--terminal-fg` は**どこからも読まれていませんでした。**
 * 容器の背景（`--terminal-bg`）と xterm が塗る背景も**別物**でした。
 * **作ったのに繋いでいない** —— Issue #10 / #14 と同じ形です。
 *
 * **色そのものは人が決める領域**です（product-baseline §11）。
 * ここがするのは**読むことと、渡すこと**だけ。値は `tokens.css` にあります。
 */

/**
 * 本文として読める下限（WCAG AA）。
 *
 * **端末に出るのは本文**です。飾りではないので、大きな文字向けの 3:1 では足りません。
 * xterm の既定をこの背景で測ると **5 色が割れます**（`black` は 1.51:1）。
 */
export const MIN_CONTRAST = 4.5;

/**
 * **端末の色の、唯一の出どころ。**
 *
 * `tokens.css` には置きません。以前は置いてあったのですが、
 * **`--terminal-fg` はどこからも読まれず**、xterm は別の値を直書きしていて、
 * **容器と端末で背景が違って**いました。**2 か所に置くと、片方だけ直ります。**
 *
 * ANSI の 16 色のうち、**割れていた 5 色だけ**を直しています。
 * **色を作り替えていません** —— 色相を保ったまま、明度を上げて基準を越える
 * 最小の値にしました（好みの入る余地を減らすため）。残り 11 色は xterm の既定です。
 *
 * **どの色にするかは人が決める領域**です（product-baseline §11 / DESIGN.md）。
 * テストが見張るのは「どの色か」ではなく、**読めない色が混ざっていないか**だけです。
 */
export const TERMINAL_PALETTE = {
	background: '#0d1013',
	foreground: '#d7dae0',
	black: '#717f84', // 既定 #2e3436 は 1.51:1 —— ほぼ見えない
	red: '#fa0000', // 既定 #cc0000 は 3.24:1
	green: '#4e9a06',
	yellow: '#c4a000',
	blue: '#487ec5', // 既定 #3465a4 は 3.22:1
	magenta: '#996ea1', // 既定 #75507b は 2.90:1
	cyan: '#06989a',
	white: '#d3d7cf',
	brightBlack: '#7b7e78', // 既定 #555753 は 2.61:1
	brightRed: '#ef2929',
	brightGreen: '#8ae234',
	brightYellow: '#fce94f',
	brightBlue: '#729fcf',
	brightMagenta: '#ad7fa8',
	brightCyan: '#34e2e2',
	brightWhite: '#eeeeec'
} as const;

/** xterm へ渡す形。**キー名は xterm の決まり**なので変えられません。 */
export interface TerminalTheme {
	background: string;
	foreground: string;
	cursor: string;
	black: string;
	red: string;
	green: string;
	yellow: string;
	blue: string;
	magenta: string;
	cyan: string;
	white: string;
	brightBlack: string;
	brightRed: string;
	brightGreen: string;
	brightYellow: string;
	brightBlue: string;
	brightMagenta: string;
	brightCyan: string;
	brightWhite: string;
}

/** `#rgb` / `#rrggbb` を 0〜1 の 3 つへ。読めなければ null。 */
function channels(colour: string): [number, number, number] | null {
	const hex = colour.trim().replace('#', '');
	const full =
		hex.length === 3
			? hex
					.split('')
					.map((one) => one + one)
					.join('')
			: hex;
	if (!/^[0-9a-fA-F]{6}$/.test(full)) return null;
	const parts = [0, 2, 4].map((at) => parseInt(full.slice(at, at + 2), 16) / 255);
	return [parts[0], parts[1], parts[2]];
}

/** WCAG の相対輝度。 */
function luminance(colour: string): number {
	const rgb = channels(colour);
	if (!rgb) return 0;
	const [r, g, b] = rgb.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** 2 色のコントラスト比。**どちらを先に渡しても同じ。** */
export function contrast(one: string, other: string): number {
	const a = luminance(one);
	const b = luminance(other);
	return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/**
 * xterm へ渡す形にする。
 *
 * **カーソルは前景と同じ。**別の色にすると、そこだけ測り直しが要ります。
 */
export function terminalTheme(): TerminalTheme {
	return { ...TERMINAL_PALETTE, cursor: TERMINAL_PALETTE.foreground };
}

/** 読めない色の一覧。**空であること**をテストが見張ります。 */
export function unreadableColours(): string[] {
	const { background, ...rest } = TERMINAL_PALETTE;
	return Object.entries(rest)
		.filter(([, colour]) => contrast(background, colour) < MIN_CONTRAST)
		.map(([name, colour]) => `${name} ${colour} ${contrast(background, colour).toFixed(2)}:1`);
}
