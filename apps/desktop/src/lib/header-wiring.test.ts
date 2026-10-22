/**
 * **ヘッダーの 2 つのボタンに、線が繋がっていること**（UI 案・2026-10-21）。
 *
 * `terminal-wiring.test.ts` と同じ考え方です —— **押しても何も起きない
 * ボタンは、型検査も試験も通り、押すまで分かりません。**
 * Issue #10 は端末がまる 1 か月、1 バイトも表示していませんでした。
 *
 * ## 画面を伏せる（D26）
 *
 * **この製品は画面そのものに接続先が写り込みます**（CLAUDE.md 禁止事項 4）。
 * 配線（塗り潰す CSS・`data-secret` の印・`data-redact` を立てる所）は
 * もともと在り、**足りなかったのは人が押す口だけ**でした。
 * それまでは写真撮り用のイベントでしか立たなかったので、
 * **人は画面共有の前に伏せられませんでした。**
 *
 * ## AI を止める
 *
 * 止めるのは**いま走っているもの**です。
 * **何もしていないときに押せるボタンを出してはいけません** ——
 * 押しても何も起きないのに「止まった」と読まれるので、**出さないより悪い。**
 */
import { describe, expect, test } from 'vitest';

// **Vite の `?raw` でソースをそのまま読みます。**`node:fs` は
// `svelte-check` の tsconfig に node の型が無く、**型検査だけが落ちます。**
import source from '../routes/+page.svelte?raw';

describe('画面を伏せる', () => {
	test('gives the person a button to press', () => {
		// それまでは `capture://redact` のイベントでしか立たなかった。
		expect(source).toContain('class="mask-toggle"');
		expect(source).toContain('onclick={toggleMask}');
	});

	test('actually flips the redaction state', () => {
		// **`redaction.set` を呼ぶこと。**`data-redact` を自分で立てると
		// 出どころが 2 つになり、**片方だけ直ります。**
		expect(source).toContain('redaction.set(!redaction.on)');
	});

	test('says out loud whether it is pressed', () => {
		// 伏せているのに伏せていないと思っている、が一番危ない。
		expect(source).toContain('aria-pressed={redaction.on}');
		expect(source).toContain('class:on={redaction.on}');
	});

	test('does not swallow a failure', () => {
		// **押したのに伏せられていない**を黙って通さない。
		expect(source).toMatch(/redaction\.set\(!redaction\.on\)\s*\.catch\(/);
	});
});

describe('AI を止める', () => {
	test('gives the person a button to press', () => {
		expect(source).toContain('class="stop-ai"');
		expect(source).toContain('onclick={() => void stopAi()}');
	});

	test('cannot be pressed while the AI is doing nothing', () => {
		// **空振りを「止めた」と見せない。**
		expect(source).toContain('disabled={!aiIsDoing}');
		expect(source).toContain("$derived(holder === 'ai' || session.busy)");
	});

	test('stops the output before closing the console', () => {
		// **順番が要る。**端末を先に閉じると、残りが流れ続けます。
		const at = source.indexOf('async function stopAi()');
		expect(at).toBeGreaterThan(-1);
		const body = source.slice(at, at + 600);
		const stream = body.indexOf("invoke('stop_stream')");
		const console_ = body.indexOf("invoke('console_stop')");
		expect(stream).toBeGreaterThan(-1);
		expect(console_).toBeGreaterThan(-1);
		expect(stream).toBeLessThan(console_);
	});

	test('only takes the console away when the AI is the one holding it', () => {
		// 人が握っている端末を、止めるボタンで閉じてしまわない。
		expect(source).toContain("if (holder === 'ai') await invoke('console_stop');");
	});
});

describe('キー操作', () => {
	test('is decided by the shared module, not here', () => {
		// 取り合いは `app-keys.ts` のテストで数えています。
		expect(source).toContain("from '$lib/app-keys'");
		expect(source).toContain('const app = appIntent(');
	});

	test('runs before the modifier check, so F1 can arrive', () => {
		// **`F1` は Ctrl も ⌘ も押しません。**下の早期 return を通ると
		// 永久に届かないので、**順番そのもの**を見張ります。
		//
		// **見るのは実行する側（`runApp`）です。**最初は `appIntent` の
		// 呼び出し位置を見ていて、**判定だけ残して実行を下へ動かす**壊し方が
		// 素通りしました（呼び出しは動かないので）。
		const run = source.indexOf('runApp(app);');
		const early = source.indexOf('if (!(event.metaKey || event.ctrlKey)) return;');
		expect(run).toBeGreaterThan(-1);
		expect(early).toBeGreaterThan(-1);
		expect(run).toBeLessThan(early);
	});

	test('does nothing on stopAi when there is nothing to stop', () => {
		expect(source).toContain('if (aiIsDoing) void stopAi();');
	});
});

describe('画面の切り替え', () => {
	test('numbers the screens from the one list that defines them', () => {
		// **案は 3 つ（端末 / ファイル / 接続）ですが、実物は 5 つです。**
		// 番号を手で書くと、画面を足した日にずれます。
		expect(source).toContain('const next = VIEWS[at];');
	});
});

describe('AI の柵（D22）', () => {
	/**
	 * **端末の面にも柵を出す**（UI 案・2026-10-21）。
	 *
	 * ファイルの面には出ていましたが、**端末の面には無かった** ——
	 * AI が端末で何かしている間、**人は囲いの大きさを見られませんでした。**
	 * D22 の約束（AI が書けるのは人が並べた所だけ・既定は空）は、
	 * **AI が動いている画面に在ってこそ意味があります。**
	 */
	test('shows the fence while the terminal is in use', () => {
		expect(source).toContain('class="fence"');
		expect(source).toContain('session.open.write.aiRoots');
	});

	test('tells the person they are not the one being fenced', () => {
		// **人は制限されません**（D22）。これを書かないと、
		// 自分が縛られていると読まれます。
		expect(source).toContain("i18n.t('files.scope.help')");
	});

	test('says plainly when the fence is empty', () => {
		// **既定は空＝AI は読むだけ。**空を「設定し忘れ」と読ませない。
		expect(source).toContain("i18n.t('files.scope.none')");
	});

	test('does not invent new wording for it', () => {
		// 言い方を増やすと、**11 言語ぶんの訳が片方だけ古くなります。**
		expect(source).not.toContain("i18n.t('console.fence");
	});

	test('colours it as the AI, not as a warning', () => {
		// **橙は AI の色。**「これは AI にかかっている制限」が色で読める。
		// 空のときは色で主張しない（安全な既定を警告に見せない）。
		expect(source).toContain('class:open={session.open.write.aiRoots.length > 0}');
	});
});

describe('帯の絞り込み（UI 案）', () => {
	/**
	 * **検査は `BandPanel.svelte.test.ts` へ移しました。**
	 *
	 * 帯を部品に出したので、**ソース文字列を見る代わりに
	 * 実際に描いて確かめられます** —— そちらが強い検査です
	 * （書き方を変えても、出るものが同じなら通る）。
	 *
	 * ここに残すのは**繋がっているかだけ**です。
	 */
	test('hands the band its lines', () => {
		expect(source).toContain('<BandPanel {lines} />');
	});

	test('does not keep a second copy of the band here', () => {
		// **写して 2 か所に置くと、片方だけ直る日が来ます**（D39）。
		expect(source).not.toContain('class="band"');
		expect(source).not.toContain('filterBand(');
	});
});

describe('キー操作を見る場所（UI 案）', () => {
	/**
	 * **入れても書いていなければ無いのと同じ**（Issue #9 で一度やった）。
	 *
	 * `Ctrl Shift H`（画面を伏せる）は、**画面共有の直前に押したい**ものです。
	 * そのとき押し方を探せないと、**接続先が写ったまま共有が始まります。**
	 */
	test('opens on F1', () => {
		expect(source).toContain("if (intent === 'help') {");
		expect(source).toContain('keyHelpOpen = true;');
	});

	test('is also reachable by pressing something', () => {
		// **ショートカットだけだと、知らない人には無いのと同じ。**
		expect(source).toContain('onclick={() => (keyHelpOpen = true)}');
	});

	test('does not send F1 to the about box any more', () => {
		// 案はヘルプと「〜について」を別に持っています。
		// 「〜について」は版を確かめる所で、**押し方の一覧ではありません。**
		const at = source.indexOf("if (intent === 'help') {");
		expect(at).toBeGreaterThan(-1);
		expect(source.slice(at, at + 120)).not.toContain('aboutOpen');
	});

	test('draws the list from the one table that defines it', () => {
		// **ヘルプが実物とずれるのが一番たちの悪い壊れ方**なので、
		// 表は `app-keys.ts` に置き、そこから描きます。
		expect(source).toContain('<KeyHelpDialog {platform}');
	});
});

describe('立ち上げ直すと AI の目も切れる（UI 案）', () => {
	/**
	 * **GUI と MCP は同じ 1 本の SSH を共有します**（禁止事項 3）。
	 *
	 * それまで数えていたのは人の側だけ（接続と端末）でした。
	 * **人が何も開いていないのに AI は読んでいる**ことは起きるので
	 * （AI が宛先を動かした直後など）、そこを数えないと
	 * **「何も切れません」と言って AI を切ります。**
	 */
	test('counts the MCP mouth when working out what a restart costs', () => {
		expect(source).toContain('mcpOpen: mcpFailure === null');
	});

	test('says the AI will not reconnect by itself', () => {
		// 黙っていると「勝手に戻る」と思われ、**戻っていないことに
		// 気づかないまま作業が進みます。**
		expect(source).toContain("i18n.t('update.ai.note')");
	});

	test('only says it when the AI is actually there', () => {
		// 居ないものについて断りを出すと、**毎回出る注意書き**になって読まれなくなります。
		expect(source).toContain('{#if cost.mcp}');
	});
});
