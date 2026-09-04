/**
 * **作った端末には、必ず書く線が繋がっていること**（Issue #10）。
 *
 * 端末タブは 2026-09-01 に入って以来、**1 バイトも表示していませんでした。**
 * 原因は 1 行です。
 *
 * ```js
 * listen('stream://raw', (event) => {
 *     if (terminal) writeChunk(terminal, event.payload);   // ← consoleTerm が無い
 * });
 * ```
 *
 * `consoleTerm` という別の xterm を作っておきながら、**そこへ書く行が
 * どこにも無かった**という形です。実体（シェル）は正常で、`read_stream` を持つ
 * AI 側からは全部見えていました。**人だけが見えていません。**
 *
 * 型検査は 1 件も止めません（`svelte-check` は 283 files, 0 errors のままでした）。
 * 部品を描いて確かめるテストがあれば止まりますが、**まだ 1 本もありません。**
 * それを入れるまでの間、**同じ穴だけでも塞いでおきます。**
 *
 * ここが見るのはソースの形です。**中身の正しさは見ていません** —
 * 「作ったのに繋がっていない」という、あの 1 つの壊れ方だけを止めます。
 */
import { describe, expect, test } from 'vitest';

// **Vite の `?raw` でソースをそのまま読みます。**`node:fs` を使うと
// `@types/node` が要り、**テスト 1 本のために依存が増えます**（product-baseline §12）。
import source from '../routes/+page.svelte?raw';

/** `xxx = createTerminal(...)` で作られた端末の名前。 */
function terminalsCreatedIn(text: string): string[] {
	const found = [...text.matchAll(/(\w+)\s*=\s*createTerminal\(/g)].map((hit) => hit[1]);
	return [...new Set(found)];
}

describe('端末の配線', () => {
	test('finds the terminals the page creates', () => {
		// **この検査自体が空振りしていないこと。**正規表現が合わなくなった日に、
		// 「1 つも見つからないので全部通った」になると、見張りが死にます。
		const created = terminalsCreatedIn(source);

		expect(created.length).toBeGreaterThanOrEqual(2);
		expect(created).toContain('consoleTerm');
	});

	test('writes the incoming stream into every terminal it creates', () => {
		// **作ったのに書かない端末を作らない。**それが Issue #10 の中身です。
		const created = terminalsCreatedIn(source);
		const unwired = created.filter((name) => !source.includes(`writeChunk(${name}`));

		expect(unwired, `作ったのに writeChunk が繋がっていない端末: ${unwired.join(', ')}`).toEqual(
			[]
		);
	});
});
