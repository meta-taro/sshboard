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
		// **押したらまず聞く**（2026-10-23）。口を閉じると、
		// 戻すにはアプリを立ち上げ直すので（§13）。
		expect(source).toContain('onclick={() => (stopAiAsking = true)}');
	});

	test('asks once before closing the mouth', () => {
		expect(source).toContain('<ConfirmDialog');
		expect(source).toContain("i18n.t('ai.stop.title')");
	});

	test('closes the MCP mouth as well, not only what is running', () => {
		// **口が空いたままだと、AI は次の呼びかけで戻ってきます** ——
		// 「止めた」と言って止まっていないのが、いちばん悪い形。
		const at = source.indexOf('async function stopAi()');
		const body = source.slice(at, at + 900);
		expect(body).toContain("invoke('mcp_stop')");
		// **口を閉じるのは最後。**先に閉じると、走っているものを止める呼びかけが届かない。
		expect(body.indexOf("invoke('stop_stream')")).toBeLessThan(body.indexOf("invoke('mcp_stop')"));
	});

	test('cannot be pressed when there is nothing to stop and no mouth open', () => {
		// **空振りを「止めた」と見せない。**
		// ただし**口が立っているなら、走っていなくても閉じる意味がある**ので、
		// そこは押せます（2026-10-23）。
		expect(source).toContain('disabled={!aiIsDoing && mcp === null}');
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
		// 走っていなくても**口が立っていれば聞く**（閉じる意味がある）。
		expect(source).toContain('if (aiIsDoing || mcp !== null) stopAiAsking = true;');
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
	 * **検査は `ConnectionRail.svelte.test.ts` へ移しました。**
	 *
	 * 柵を部品に出したので、**ソース文字列を見る代わりに
	 * 実際に描いて確かめられます**（空を警告色にしない・道を全部出す・
	 * 「人は制限されない」を添える、など 7 本）。
	 *
	 * ここに残すのは**繋がっているかだけ**です。
	 */
	test('hands the rail the fence of the connection in view', () => {
		expect(source).toContain('roots={session.open?.write.aiRoots ?? []}');
	});

	test('does not keep a second copy of the fence here', () => {
		// **写して 2 か所に置くと、片方だけ直る日が来ます**（D39）。
		expect(source).not.toContain('class="fence"');
		expect(source).not.toContain('function fenceHead');
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

describe('3 つの列（UI 案・2026-10-21）', () => {
	/**
	 * **帯を常に見えるようにする**のが要点です（PRD §4-1
	 * 「見えないことが最大の危険」）—— 端末を打っている人が、
	 * **AI が何をしたかを見るために画面を移らなくていい。**
	 *
	 * **配置は型検査もテストも素通りします。**描いて確かめても、
	 * jsdom には幅が無いので列の形は測れません（`icon-shape.test.ts` と同じ事情）。
	 * **測れないものを測ったことにしない**ため、ここでは「書いてあること」を見ます。
	 */
	test('puts the three columns in one frame', () => {
		expect(source).toContain('<div class="three">');
		expect(source).toContain('<div class="middle">');
	});

	test('keeps the band beside the terminal, not only in its own view', () => {
		// **ここが案の要点。**画面を移らずに AI の手が見える。
		const at = source.indexOf('<div class="three">');
		const end = source.indexOf('</section>', at);
		expect(at).toBeGreaterThan(-1);
		expect(source.slice(at, end)).toContain('<BandPanel {lines} />');
	});

	test('stands the connection rail up on its side', () => {
		// 横並びのままだと、左の列になりません。
		expect(source).toContain('layout="rail"');
	});

	test('does not give the columns a fixed width', () => {
		// **固定幅で並べると、窓を狭めた人の端末が消えます。**
		// 折り返して縦に積む（消えるのではなく、下に来る）。
		const at = source.indexOf('.three {');
		expect(at).toBeGreaterThan(-1);
		const rule = source.slice(at, source.indexOf('}', at));
		expect(rule).toContain('flex-wrap: wrap');
	});

	test('gives the middle column the leftover room', () => {
		// 中央が主役。両脇が先に縮む。
		expect(source).toContain('flex: 999 1 22rem');
	});

	test('lets the terminal shrink instead of pushing the columns apart', () => {
		// `min-width: 0` が無いと、**端末の中身が列を押し広げて
		// 横スクロールが出ます。**
		const at = source.indexOf('.three .middle {');
		const rule = source.slice(at, source.indexOf('}', at));
		expect(rule).toContain('min-width: 0');
	});
});

describe('メニューはロゴの下（UI 案・2026-10-21）', () => {
	test('opens from the logo', () => {
		expect(source).toContain('class="brand"');
		expect(source).toContain('aria-expanded={viewMenuOpen}');
	});

	test('says it opens something, before it is pressed', () => {
		// **落としの印を出す。**押すまで分からない口は、無いのと同じ。
		expect(source).toContain('class="brand-chevron"');
	});

	test('holds help, diagnostics and about', () => {
		// 案は「このアプリ」の群をここに置きました。
		expect(source).toContain("keyHelpOpen = true; }}");
		expect(source).toContain("humanShows('diag'); }}");
		expect(source).toContain("aboutOpen = true; }}");
	});

	test('closes itself when one of them is chosen', () => {
		// 開いたままだと、出した先が隠れます。
		expect(source).toContain('viewMenuOpen = false; keyHelpOpen = true;');
	});

	test('keeps the help button in the header', () => {
		// **案もそうしています。**押し方を探している人が、
		// メニューを開かなくていい（Issue #9 —— 隠れているものは無いのと同じ）。
		expect(source).toContain('onclick={() => (keyHelpOpen = true)}');
	});

	test('does not keep a second about button outside the menu', () => {
		// **口を 2 つ置くと、片方だけ直る日が来ます**（D39）。
		expect(source).not.toContain('onclick={() => (aboutOpen = true)}');
	});

	test('leaves enough of the bar to grab the window by', () => {
		// この帯は OS のタイトルバー（D17）。**34px では窓を動かせない**と
		// 実機から言われています。
		//
		// 以前は「ロゴが縮む」ことで掴む所を残していましたが、**製品名が
		// 「sshb…」に切れる**写真が出ました（2026-10-24）。掴む所は
		// `header` 全体が `data-tauri-drag-region` なので、ロゴを縮める必要はありません。
		expect(source).toContain('<header data-tauri-drag-region');
		const at = source.indexOf('.brand {');
		const rule = source.slice(at, source.indexOf('}', at));
		expect(rule).toContain('flex: none');
	});
});

describe('狭い窓でも、閉じる釦と MCP の釦は消えない（Windows 実機・2026-10-25）', () => {
	/**
	 * 0.1.27 で、**Windows の窓から右上の × が消え、MCP の釦が見えなくなりました。**
	 *
	 * - 製品名とタブを縮めない形にしたので、狭い窓では帯が折り返し、
	 *   **窓の操作（─ □ ×）が 2 段目の左端へ落ちた**（900px で計測: 高さ 13px）
	 * - 1240px 未満で MCP の文言を隠したので、**釦が印だけになり、MCP と読めなくなった**
	 *
	 * Windows の窓は、表示倍率のせいで 1240px（論理）を下回るのが普通です。
	 */
	function rule(selector: string): string {
		const at = source.indexOf(`\n\t${selector} {`);
		expect(at, `${selector} の規則が無い`).toBeGreaterThan(-1);
		return source.slice(at, source.indexOf('\n\t}', at));
	}

	test('pins the window controls to the top right, whatever wraps', () => {
		// **折り返しの流れに乗せない。**乗せると、帯が折れた日に × が逃げます。
		expect(rule('.window-controls')).toContain('position: absolute');
		expect(rule('.window-controls')).toContain('right: 0');
		expect(rule('header')).toContain('position: relative');
	});

	test('keeps room on the right so nothing slides under the controls', () => {
		expect(rule('header')).toContain('padding-right: var(--window-controls-w)');
	});

	test('never lets the MCP button become a bare icon', () => {
		// 文言を隠すなら、**短い言い方を出す。**印だけでは何の釦か分かりません。
		expect(source).toContain('class="mcp-short"');
		const narrow = source.slice(source.indexOf('@media (max-width: 1240px)'));
		expect(narrow).toMatch(/\.mcp \.mcp-short \{\s*display: inline;/);
	});
});

describe('AI の提案と、人の登録（実運用の報告・2026-10-23）', () => {
	/**
	 * **これは `run_command` ではありません。**
	 *
	 * 本番で、調べるところは sshboard で完結したのに直す段で止まり、
	 * **操作が画面の外で行われ、記録も残りませんでした。**
	 * `run_command` を作らない決定（D3）が守ろうとしたもの
	 * 「人の見ていない所で終わらせない」が裏返った形です。
	 *
	 * 足した道が同じ穴にならないよう、ここで数えます。
	 */
	test('writes the file through the GUI, never through the AI', () => {
		// **AI にこの口はありません**（MCP には出していない）。
		expect(source).toContain("invoke('operation_register', { proposal })");
	});

	test('keeps registering and running apart', () => {
		// **登録しても走りません。**走るときは承認の問い（`operationAsk`）を通る。
		// 1 回の押下にまとめたら、それは手順を増やした `run_command`。
		expect(source).toContain('let operationProposal = $state<OperationProposal | null>(null);');
		expect(source).toContain('let operationAsk = $state<OperationAsk | null>(null);');
	});

	test('does not swallow a failure when registering', () => {
		// **押したのに入っていない**を黙って通さない。
		const at = source.indexOf('async function registerProposal');
		expect(at).toBeGreaterThan(-1);
		expect(source.slice(at, at + 420)).toContain('failure = String(error)');
	});

	test('writes nothing when the person says no', () => {
		expect(source).toContain("invoke('operation_dismiss')");
	});

	test('listens for proposals so they cannot be missed', () => {
		// 提案が人の目に触れないと、操作は画面の外へ出ます。
		expect(source).toContain("listen<OperationProposal | null>('operation://proposal'");
		// **起動直後の取りこぼし対策。**購読だけだと、出たあとに開いた人には届かない。
		expect(source).toContain("invoke<OperationProposal | null>('operation_proposal')");
	});
});

describe('立ち上げ直したあとの繋ぎ直し（UI 案・5 枚目）', () => {
	test('remembers what was open before restarting', () => {
		// 控えるのは**識別子だけ**（`reopen.ts` が中身を見張っています）。
		expect(source).toContain('updater.rememberOpen(session.all.map((held) => held.id))');
	});

	test('asks once on the next start, instead of reconnecting by itself', () => {
		// **勝手に繋ぎ直すと、人が見ていない所で SSH が開きます**（禁止事項 3）。
		expect(source).toContain('reopenAsk = takeReopen();');
		expect(source).toContain("i18n.t('reopen.title')");
	});

	test('names which connections it is about', () => {
		// 「繋ぎ直しますか」だけでは、**何に繋ぐのか読めません。**
		expect(source).toContain("reopenAsk.ids.join(' , ')");
	});

	test('forgets the note when the person says no', () => {
		// 残すと、起動のたびに同じことを聞かれます。
		expect(source).toContain('forgetReopen();');
	});

	test('tries the rest even if one fails, and still says why', () => {
		// **1 本落ちても残りは試す。**そのうえで理由は出す（握り潰さない）。
		const at = source.indexOf('async function reopenThem');
		const body = source.slice(at, at + 600);
		expect(body).toContain('for (const id of ids)');
		expect(body).toContain('failure = String(error)');
	});

	test('does not reconnect the AI', () => {
		// **AI の分は繋ぎ直しません**（AI のほうから呼ばれたときに繋がる）。
		const at = source.indexOf('async function reopenThem');
		const body = source.slice(at, at + 600);
		expect(body).not.toContain('console_open');
		expect(body).not.toContain('Actor::Ai');
	});
});
