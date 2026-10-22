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

// **Vite の `?raw`。**`node:fs` は型検査が落ちます（`icon-shape.test.ts` の注）。
import page from '../routes/+page.svelte?raw';
import layout from '../routes/+layout.svelte?raw';

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

describe('端末の背景の出どころ（2026-10-22）', () => {
	/**
	 * **容器と端末で背景が違う**という形を、もう一度作らないため。
	 *
	 * `tokens.css` の注にあるとおり、以前は `--terminal-bg` をトークンに置き、
	 * xterm は別の値を直書きしていて、**容器と端末で背景が違っていました。**
	 * いまは `TERMINAL_PALETTE` が唯一の出どころで、
	 * `+layout.svelte` が実行時に `--terminal-bg` へ入れています。
	 *
	 * **ただし `+page.svelte` に控えの値が直書きされています**
	 * （`var(--terminal-bg, #0d1013)`）——最初の 1 描画のためのもので、
	 * **いまはたまたま一致しています。**`TERMINAL_PALETTE` を変えた日に
	 * 黙ってずれる（一瞬だけ違う色が見える）ので、ここで数えます。
	 */
	test('keeps the first-paint fallback equal to the palette', () => {
		const hit = /var\(--terminal-bg,\s*(#[0-9a-fA-F]{6})\)/.exec(page);
		expect(hit, '`var(--terminal-bg, …)` の控えが見つかりません').not.toBeNull();
		expect(hit?.[1].toLowerCase()).toBe(TERMINAL_PALETTE.background.toLowerCase());
	});

	test('still has one place that fills the variable', () => {
		// **控えだけになっていないこと。**入れる所が消えると、
		// テーマを変えても端末の容器だけ古い色のままになります。
		expect(layout).toContain("setProperty('--terminal-bg', TERMINAL_PALETTE.background)");
	});
});
