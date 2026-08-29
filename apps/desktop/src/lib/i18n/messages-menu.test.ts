/**
 * OS のメニューの文言。**訳が 1 つでも抜けると、その項目が空欄で出ます。**
 *
 * Rust 側は画面が渡した文字列でメニューを組むので（`menu.rs`）、
 * ここが欠けると**押せるのに名前の無い項目**になります。
 */
import { describe, expect, test } from 'vitest';

import { LOCALES } from './locales';
import { MENU, MENU_KEYS, menuLabels } from './messages-menu';

describe('the menu catalogue', () => {
	test('covers every language the picker offers', () => {
		// 言語を足したのにメニューだけ英語のまま、を起こさない。
		for (const locale of LOCALES) {
			expect(MENU[locale.code], `${locale.code} のメニューが無い`).toBeDefined();
		}
	});

	test('has every key in every language, with nothing blank', () => {
		for (const [code, catalog] of Object.entries(MENU)) {
			for (const key of MENU_KEYS) {
				expect(catalog[key], `${code} の ${key} が無い`).toBeTruthy();
				expect(catalog[key].trim(), `${code} の ${key} が空`).not.toBe('');
			}
		}
	});

	test('includes the view menu, which is where people look for text size', () => {
		// **右上のボタンだけだと見つからなかった**（実際に見つけてもらえなかった）。
		expect(MENU_KEYS).toContain('menu.view');
		expect(MENU_KEYS).toContain('menu.textLarger');
		expect(MENU_KEYS).toContain('menu.textSmaller');
		expect(MENU_KEYS).toContain('menu.textReset');
	});
});

describe('menuLabels', () => {
	test('falls back to English for a language it does not know', () => {
		expect(menuLabels('xx-YY')).toBe(MENU.en);
	});

	test('returns the asked-for language when it has one', () => {
		expect(menuLabels('ja')['menu.view']).toBe('表示');
	});
});
