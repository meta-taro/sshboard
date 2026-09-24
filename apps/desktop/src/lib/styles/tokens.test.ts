/**
 * **使っている色の名前が、本当に定義されているか。**
 *
 * `var(--typo)` は**黙って当たりません。**エラーも警告も出ず、型検査も試験も通り、
 * **画面を見るまで分かりません。**地の色が当たらなければ親の色がそのまま出るので、
 * 「なんとなく地味」にしか見えません。
 *
 * 割符の席が同じ穴で 4 件出しました（2026-09-24）——
 *
 * > `var(--bg)` は無い（在るのは `--bg-app` / `-subtle` / `-sunken` / `-elevated`）
 * > **地も文字色も当たらず、選んでいる側が周りと同じ見た目になっていました。**
 * > 残っていたのは `:hover` だけなので、**触ったほうが選ばれて見えた。**
 *
 * **こちらも同じ日に、同じ所を踏んでいます** —— 暗い配色で選ばれているタブが
 * 帯と差 6 / 255 しか無く、見分けが付きませんでした（`--surface-raised` で解消）。
 * **あれは名前の間違いではありませんでしたが、症状は同じでした。**
 *
 * ## 見張らないもの
 *
 * - **控えの在る `var(--x, 既定)`** —— 無くても当たるので、間違いではない
 * - **注記の中** —— 説明に書いた名前を「使っている」と読まない
 * - **組み立てた名前**（`` var(--mark-${color}) ``）—— 静的には解けない
 */
import { describe, expect, test } from 'vitest';

/**
 * **Vite の glob に頼りません。**`{svelte,css}` の波括弧は展開されず、
 * 分けて書いても `.css` は 1 件も拾えませんでした（2026-09-24）。
 * **静かに 0 件になる読み方**は、この試験がいちばん避けたいものです。
 * ファイルとして自分で歩きます。
 */
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const SRC = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

function walk(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = join(dir, entry.name);
		if (entry.isDirectory()) return walk(full);
		return /\.(svelte|css)$/.test(entry.name) && !entry.name.includes('.test.') ? [full] : [];
	});
}

const sources: Record<string, string> = Object.fromEntries(
	walk(SRC).map((full) => [relative(SRC, full), readFileSync(full, 'utf8')])
);

const cssCount = Object.keys(sources).filter((p) => p.endsWith('.css')).length;
const svelteCount = Object.keys(sources).filter((p) => p.endsWith('.svelte')).length;

/** 注記を外す。**説明に書いた名前を、使っていると読まないため。** */
function stripNotes(text: string): string {
	return text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/<!--[\s\S]*?-->/g, '');
}

type Scan = { defined: Set<string>; used: Map<string, Set<string>> };

function scan(files: Record<string, string>): Scan {
	const defined = new Set<string>();
	const used = new Map<string, Set<string>>();
	for (const [path, raw] of Object.entries(files)) {
		if (path.includes('.test.')) continue;
		const text = stripNotes(raw);
		// CSS の定義と、`style="--x: …"` の両方がこの形。
		for (const m of text.matchAll(/(--[A-Za-z0-9_-]+)\s*:/g)) defined.add(m[1]);
		// Svelte の `style:--x={…}`。**これも定義。**
		for (const m of text.matchAll(/style:(--[A-Za-z0-9_-]+)/g)) defined.add(m[1]);
		for (const m of text.matchAll(/var\(\s*(--[A-Za-z0-9_-]+)\s*([,)])/g)) {
			if (m[2] === ',') continue; // 控えが在る
			if (!used.has(m[1])) used.set(m[1], new Set());
			used.get(m[1])!.add(path);
		}
	}
	return { defined, used };
}

function missing({ defined, used }: Scan): string[] {
	return [...used.entries()]
		.filter(([name]) => !defined.has(name))
		.map(([name, where]) => `${name}  ←  ${[...where].sort().join(', ')}`)
		.sort();
}

describe('色の名前', () => {
	/**
	 * **0 件だから通った、と「見張る先が空だから通った」を区別する。**
	 * ここが空になったまま緑になるのが、いちばん質の悪い壊れ方です。
	 */
	test('見張る先が空になっていない', () => {
		const { defined, used } = scan(sources);
		expect(Object.keys(sources).length).toBeGreaterThan(10);
		expect(defined.size).toBeGreaterThan(20);
		expect(used.size).toBeGreaterThan(10);
		// **色の束が読めていないまま緑になるのが、いちばん質の悪い通り方です。**
		expect(cssCount).toBeGreaterThan(0);
		expect(svelteCount).toBeGreaterThan(5);
	});

	/** **落ちる側を先に確かめる。**植えたものを見つけられなければ、試験に意味がない。 */
	test('定義に無い名前を、実際に見つけられる', () => {
		const planted = {
			'/fake/a.css': ':root { --real: #fff; }',
			'/fake/b.svelte': '<style>.x { color: var(--real); background: var(--not-defined); }</style>'
		};
		expect(missing(scan(planted))).toEqual(['--not-defined  ←  /fake/b.svelte']);
	});

	test('控えの在る `var(--x, 既定)` は落とさない', () => {
		const planted = {
			'/fake/c.svelte': '<style>.x { background: var(--nowhere, #0d1013); }</style>'
		};
		expect(missing(scan(planted))).toEqual([]);
	});

	test('注記の中の名前を、使っていると読まない', () => {
		const planted = {
			'/fake/d.css': '/* var(--only-in-a-note) のことは数えない */ :root { --a: 1px; }'
		};
		expect(missing(scan(planted))).toEqual([]);
	});

	test('**使っているのに定義が無い名前は 1 つも無い**', () => {
		expect(missing(scan(sources))).toEqual([]);
	});
});
