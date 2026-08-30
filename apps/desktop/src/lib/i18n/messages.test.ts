/**
 * 画面の文言。**英語が正本で、他の言語はそこへ落ちます。**
 *
 * 落ちること自体は事故ではありませんが、**日本語で使っているのに英語が出る**のは
 * 実際に起きました。ここで欠けを数え、増えたら気づけるようにします。
 */
import { describe, expect, test } from 'vitest';

import { LOCALES } from './locales';
import { CATALOGS, FALLBACK } from './messages';

const KEYS = Object.keys(FALLBACK) as Array<keyof typeof FALLBACK>;

describe('the message catalogue', () => {
	test('covers every language the picker offers', () => {
		for (const locale of LOCALES) {
			expect(CATALOGS[locale.code], `${locale.code} のカタログが無い`).toBeDefined();
		}
	});

	test('has no blank string anywhere', () => {
		// 空文字は落ちずにそのまま出る。**ボタンの名前が消える。**
		for (const [code, catalog] of Object.entries(CATALOGS)) {
			for (const [key, value] of Object.entries(catalog)) {
				expect(String(value).trim(), `${code} の ${key} が空`).not.toBe('');
			}
		}
	});

	test('translates every key in every language', () => {
		// **英語へ落ちるのは最後の手段。**日本語で使っているのに英語が出るのは、
		// 訳の入れ忘れであって仕様ではない。
		const missing: string[] = [];
		for (const [code, catalog] of Object.entries(CATALOGS)) {
			if (code === 'en') continue;
			for (const key of KEYS) {
				if (!(key in catalog)) missing.push(`${code}: ${key}`);
			}
		}
		expect(missing, `訳が抜けている: ${missing.slice(0, 10).join(' / ')}`).toEqual([]);
	});

	test('keeps the placeholders identical to the English original', () => {
		// `{count}` を訳し忘れると、**数字が出ないまま文が成立してしまう。**
		const placeholders = (text: string) => (text.match(/\{[a-z]+\}/gi) ?? []).sort();

		for (const [code, catalog] of Object.entries(CATALOGS)) {
			if (code === 'en') continue;
			for (const key of KEYS) {
				const translated = catalog[key];
				if (translated === undefined) continue;
				expect(placeholders(translated), `${code} の ${key} の差し込みが違う`).toEqual(
					placeholders(FALLBACK[key])
				);
			}
		}
	});
});

describe('the first-run guidance', () => {
	test('exists, because people got stuck without it', () => {
		// **初動で迷わせない。**実際に迷わせた。
		for (const key of ['files.step1', 'files.step2', 'files.step3'] as const) {
			expect(KEYS, `${key} が無い`).toContain(key);
		}
	});
});

describe('the text that actually reaches the screen', () => {
	test('carries no Markdown, because nothing renders it', () => {
		// このリポジトリの文書は Markdown で書くので、**コードのコメントと同じ気分で
		// 強調を書いてしまう。**画面はそれをそのまま出すため、
		// アスタリスクが見えたままになる（**実際になった**・2026-08-30）。
		//
		// バッククォートは見張らない。**平文でもコマンドを囲む慣習として通じる**ので、
		// `ssh-keygen -lf` のような書き方はそのままでよい。
		const leaking: string[] = [];
		for (const [code, catalog] of Object.entries(CATALOGS)) {
			for (const [key, value] of Object.entries(catalog)) {
				if (String(value).includes('**')) leaking.push(`${code}: ${key}`);
			}
		}
		expect(leaking, `記号がそのまま出ます: ${leaking.join(' / ')}`).toEqual([]);
	});
});
