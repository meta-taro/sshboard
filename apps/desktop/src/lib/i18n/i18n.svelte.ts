/**
 * 表示言語。**既定は OS の言語**で、選んだら残します。
 *
 * 訳が無い鍵は英語へ落ちます。**画面に鍵がそのまま出るよりはまし**だからです。
 */
import { invoke } from '@tauri-apps/api/core';

import { CATALOGS, FALLBACK, type MessageKey } from './messages';
import { DEFAULT_LOCALE, preferredLocale, SUPPORTED_CODES } from './locales';
import { MENU_KEYS, menuLabels } from './messages-menu';

const STORAGE_KEY = 'sshboard-locale';

class I18n {
	locale = $state<string>(DEFAULT_LOCALE);

	init(): void {
		if (typeof window === 'undefined') return;

		const stored = localStorage.getItem(STORAGE_KEY);
		this.locale = preferredLocale(stored, navigator.language ?? null);
		this.#applyLang();
		this.#applyMenu();
	}

	set(code: string): void {
		if (!SUPPORTED_CODES.includes(code)) return;
		this.locale = code;
		if (typeof window !== 'undefined') localStorage.setItem(STORAGE_KEY, code);
		this.#applyLang();
		this.#applyMenu();
	}

	/**
	 * 文字列を引く。`{name}` は `vars` で置き換えます。
	 *
	 * **鍵が無いときは鍵をそのまま返します。**空文字を返すと、
	 * 抜けている場所が画面から見えなくなるためです。
	 */
	t(key: MessageKey, vars?: Record<string, string | number>): string {
		const catalog = CATALOGS[this.locale] ?? {};
		const template = catalog[key] ?? FALLBACK[key] ?? key;

		if (!vars) return template;
		return Object.entries(vars).reduce(
			(text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
			template
		);
	}

	/**
	 * OS のメニューバーを組み直す。**訳はここから渡す。**
	 * Rust 側にも訳を置くと、片方だけ直す事故が起きます。
	 *
	 * **項目を手で並べないこと。**以前ここを手書きしていて、
	 * 「表示」メニューを足したときに渡し忘れ、**メニュー全体が既定の英語へ戻った。**
	 * `MENU_KEYS` から機械的に作れば、鍵を足した時点で必ず渡ります。
	 */
	#applyMenu(): void {
		const labels = menuLabels(this.locale);
		// `menu.textLarger` → `textLarger`。Rust 側の MenuLabels は camelCase。
		const forRust = Object.fromEntries(
			MENU_KEYS.map((key) => [key.slice('menu.'.length), labels[key]])
		);

		invoke('set_menu_labels', { labels: forRust }).catch((error: unknown) => {
			// **黙らない。**組めなかったことに気づけないと、既定の英語メニューのまま
			// 使い続けることになる（実際になった）。画面は使えるのでアプリは止めない。
			console.error('[sshboard] メニューを組めません', error);
		});
	}

	/** 読み上げと字体の選択のために、`<html lang>` を合わせる。 */
	#applyLang(): void {
		if (typeof document === 'undefined') return;
		document.documentElement.setAttribute('lang', this.locale);
	}
}

export const i18n = new I18n();
