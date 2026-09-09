/**
 * 端末の配色。**読めない色を 1 つも残さない。**
 *
 * 端末の配色は Phase 0 の仮置き（`PLACEHOLDER_THEME`）のままでした。
 * しかも `tokens.css` に `--terminal-fg` があるのに**どこからも読まれておらず**、
 * 容器の背景（`#0d1013`）と xterm が塗る背景（`#16181d`）が**別物**でした。
 * **作ったのに繋いでいない** —— Issue #10 / #14 と同じ形です。
 *
 * そして **ANSI の 16 色を 1 つも定義していません**でした。xterm の既定が
 * そのまま出ており、この背景に対して測ると **5 色が本文の基準を割ります。**
 *
 * ```
 * black        #2e3436   1.51:1   ← ほぼ見えない
 * red          #cc0000   3.24:1
 * blue         #3465a4   3.22:1
 * magenta      #75507b   2.90:1
 * brightBlack  #555753   2.61:1
 * ```
 *
 * **色そのものは人が決める領域**です（product-baseline §11）。
 * ここが見張るのは「どの色か」ではなく、**読めない色が混ざっていないか**だけです。
 */
import { describe, expect, test } from 'vitest';

import {
	contrast,
	MIN_CONTRAST,
	TERMINAL_PALETTE,
	terminalTheme,
	unreadableColours
} from './terminal-theme';

describe('contrast', () => {
	test('measures the two ends correctly', () => {
		expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 1);
		expect(contrast('#123456', '#123456')).toBeCloseTo(1, 5);
	});

	test('does not care which way round it is given', () => {
		expect(contrast('#0d1013', '#d7dae0')).toBeCloseTo(contrast('#d7dae0', '#0d1013'), 5);
	});

	test('does not pretend an unreadable colour is readable', () => {
		// **これが実際に出ていた値です。**`black` は既定のままだと
		// この背景で 1.51:1 —— **その色で書かれた文字は人に見えません。**
		expect(contrast('#0d1013', '#2e3436')).toBeLessThan(MIN_CONTRAST);
	});
});

describe('端末の配色', () => {
	test('leaves no colour a person cannot read', () => {
		// **これが本題です。**xterm の既定に任せると、この背景に対して
		// **測られていない色**が混ざります（実測で 5 色が割れていました）。
		const failed = unreadableColours();

		expect(
			failed,
			`背景 ${TERMINAL_PALETTE.background} に対して読めない色があります（基準 ${MIN_CONTRAST}:1）: ${failed.join(' / ')}`
		).toEqual([]);
	});

	test('defines all sixteen ANSI colours, not just a foreground', () => {
		// 足りない色は xterm の既定へ落ちます。**落ちた分は測られていません。**
		for (const name of [
			'black',
			'red',
			'green',
			'yellow',
			'blue',
			'magenta',
			'cyan',
			'white',
			'brightBlack',
			'brightRed',
			'brightGreen',
			'brightYellow',
			'brightBlue',
			'brightMagenta',
			'brightCyan',
			'brightWhite'
		]) {
			expect(TERMINAL_PALETTE, `${name} が無い`).toHaveProperty(name);
		}
	});

	test('hands xterm the same colours, with the cursor following the foreground', () => {
		const theme = terminalTheme();

		expect(theme.background).toBe(TERMINAL_PALETTE.background);
		expect(theme.black).toBe(TERMINAL_PALETTE.black);
		// **カーソルだけ別の色にしない。**そこだけ測り直しが要ります。
		expect(theme.cursor).toBe(TERMINAL_PALETTE.foreground);
	});
});
