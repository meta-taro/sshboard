/**
 * 出す言語。**それぞれの言語が自分を呼ぶ名前**を並べます。
 * いま何語で表示していても、切替の一覧が読めるようにするためです。
 *
 * dbboard の `locales.ts` からの移植（D5）。
 */
export interface LocaleMeta {
	code: string;
	/** その言語が自分を呼ぶ名前。 */
	native: string;
}

export const LOCALES: LocaleMeta[] = [
	{ code: 'en', native: 'English' },
	{ code: 'ja', native: '日本語' },
	{ code: 'ko', native: '한국어' },
	{ code: 'zh-CN', native: '简体中文' },
	{ code: 'zh-TW', native: '繁體中文' },
	{ code: 'de', native: 'Deutsch' },
	{ code: 'fr', native: 'Français' },
	{ code: 'es', native: 'Español' },
	{ code: 'pt-BR', native: 'Português (Brasil)' },
	{ code: 'ru', native: 'Русский' },
	{ code: 'it', native: 'Italiano' }
];

export const DEFAULT_LOCALE = 'en';

export const SUPPORTED_CODES: string[] = LOCALES.map((locale) => locale.code);

/**
 * 任意の言語タグ（`navigator.language` など）を、出せる言語へ寄せる。
 *
 * 完全一致 → 主部分の一致（`ja-JP` → `ja`）→ 地域付きの前方一致（`zh` → `zh-CN`）。
 * **どれにも当たらなければ `null`。**当たらないものを無理に当てない。
 */
export function resolveLocale(tag: string | null | undefined): string | null {
	if (!tag) return null;
	if (SUPPORTED_CODES.includes(tag)) return tag;

	const primary = tag.split('-')[0].toLowerCase();
	const exact = SUPPORTED_CODES.find((code) => code.toLowerCase() === primary);
	if (exact) return exact;

	const prefixed = SUPPORTED_CODES.find((code) => code.toLowerCase().startsWith(primary + '-'));
	return prefixed ?? null;
}

/**
 * 出す言語を決める。**保存した選択 → OS の言語 → 既定**の順。
 *
 * この版で出せない言語は**飛ばします。**次の版で言語を減らしたときに、
 * 画面がメッセージの鍵だらけになるのを防ぐためです。
 */
export function preferredLocale(stored: string | null, osLanguage: string | null): string {
	if (stored && SUPPORTED_CODES.includes(stored)) return stored;
	return resolveLocale(osLanguage) ?? DEFAULT_LOCALE;
}
