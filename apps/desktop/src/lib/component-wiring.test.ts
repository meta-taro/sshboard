/**
 * **作った部品は、必ずどこかから描かれていること**（Issue #10 の再発防止）。
 *
 * #10 は「`consoleTerm` を作ったのに、書く線がどこにも無かった」でした。
 * 同じ形の失敗が、部品でも起きます — **作って、繋ぎ忘れる。**
 * 実際に `redaction.svelte.ts` が「書いたまま誰も呼んでいない」状態で
 * しばらく残っていました（`+layout.svelte` から繋いで解消）。
 *
 * 型検査は 1 件も止めません。**使われていない部品は、ただ静かに存在します。**
 *
 * ここが見るのはソースの形だけです。**描かれる条件が正しいかは見ていません。**
 * 止めるのは「1 か所からも参照されていない」という、あの 1 つの壊れ方です。
 */
import { describe, expect, test } from 'vitest';

const components = import.meta.glob('./components/*.svelte', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const sources = import.meta.glob('../**/*.{svelte,ts}', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** `./components/Foo.svelte` → `Foo` */
function nameOf(path: string): string {
	return path.slice(path.lastIndexOf('/') + 1).replace('.svelte', '');
}

describe('部品の配線', () => {
	test('finds the components that exist', () => {
		// **この検査自体が空振りしていないこと。**
		const names = Object.keys(components).map(nameOf);

		expect(names.length).toBeGreaterThanOrEqual(8);
		expect(names).toContain('ConsoleRequestDialog');
	});

	test('renders every component it defines somewhere', () => {
		const orphans = Object.keys(components)
			.map(nameOf)
			.filter((name) => {
				// 自分自身のファイル以外で `<Name` として描かれているか。
				return !Object.entries(sources).some(
					([path, text]) => !path.endsWith(`${name}.svelte`) && text.includes(`<${name}`)
				);
			});

		expect(orphans, `作ったのに、どこからも描かれていない部品: ${orphans.join(', ')}`).toEqual([]);
	});
});

/**
 * **宛先が決まったら、サーバー側の一覧を読むこと**（2026-09-18 に実機で踏みました）。
 *
 * 配布ページ用の写真を撮っていて出ました。接続を開いて［ファイル］を開くと、
 * **サーバー側が「空です」**のまま。実際には 6 件在りました。
 *
 * ```text
 * MCP  : upload, app, .ssh, .bashrc, .bash_profile, .bash_logout
 * 画面 : 空です。
 * ```
 *
 * `onMount` は `loadLocal()`（手元）を呼びますが、**`refresh()`（サーバー側）を
 * 呼んでいませんでした。**しかもタブが 1 本だけだと押しても何も起きません
 * （`switchTo` は `id === session.activeId` で素通りする）ので、
 * **人は F5 か［↻］を押すまで、空だと信じることになります。**
 *
 * **「読んでいない」と「空」を同じ顔で出すのが、この製品で一番悪い形**です。
 */
describe('ファイルの面', () => {
	const browser = components['./components/FileBrowser.svelte'];

	test('finds the file browser', () => {
		expect(browser).toBeTruthy();
	});

	test('reads the server side when the target becomes known', () => {
		// **`session.activeId` を見て読み直す線が在ること。**
		// 在れば、人がタブを押したときも、**AI が宛先を動かしたときも**追従します。
		const watches = /\$effect\(\(\) => \{[\s\S]*?session\.activeId[\s\S]*?\}\)/.test(browser);

		expect(watches).toBe(true);
	});
});

/**
 * **問いの題字が、本文へ重ならないこと**（2026-09-18 に実機で踏みました）。
 *
 * 端末の同意ダイアログを英語で撮ったら、こうなっていました ——
 *
 * ```text
 * The AI wants to use the
 * console          ← **本文の 1 行目に重なっている**
 * If you allow it, the AI can type into this terminal.
 * ```
 *
 * **日本語では出ません。**「AI が端末を使いたいと言っています」は 1 行に収まります。
 * **英語は語が長い**ので、そこだけ折れます。
 *
 * `header { align-items: center }` は**題字が 1 行である前提**の置き方です。
 * 2 行になったとき、**アイコンを中央に合わせようとして高さが伸びません。**
 * **`flex-start` なら、アイコンは 1 行目に付き、題字は素直に下へ伸びます。**
 *
 * **ここは「AI に端末を渡す唯一の同意画面」**です（D42）。
 * **読めない同意は、同意ではありません。**
 */
describe('問いの題字', () => {
	const dialogs = Object.entries(components).filter(([path]) => /Dialog\.svelte$/.test(path));

	test('finds the dialogs', () => {
		expect(dialogs.length).toBeGreaterThanOrEqual(4);
	});

	test('does not lay the heading out with flex', () => {
		// **`align-items` をどちらにしても直りませんでした**（2026-09-21）。
		// **WebKit では flex 容器の高さが中身に追いつかない場面がある** ——
		// 接続タブの名前が切れたのと同じ族です。
		//
		// **アイコンを題字の中へ入れて、普通の行の流れにする。**
		// 行が増えれば高さも増える —— **仕組みで保証されます。**
		const flexed = dialogs
			.filter(([, source]) => {
				const rule = source
					.replace(/\/\*[\s\S]*?\*\//g, '')
					.match(/\n\theader \{[\s\S]*?\n\t\}/)?.[0];
				return rule ? /display:\s*flex/.test(rule) : false;
			})
			.map(([path]) => nameOf(path));

		expect(flexed).toEqual([]);
	});

	test('does not stack the dialog body with flex either', () => {
		// **今日 3 度目の同じ形**（2026-09-21）。
		// `.dialog { display: flex; flex-direction: column; gap }` で積んでいたとき、
		// **本文が 3 行に折れると、その下の要素と間が詰まって接触しました。**
		//
		// 題字（`3563a1d`）・接続タブ（`22fd621`）と同じ ——
		// **WebKit では flex 容器の高さが中身に追いつかない場面がある。**
		const flexed = dialogs
			.filter(([, source]) => {
				const rule = source
					.replace(/\/\*[\s\S]*?\*\//g, '')
					.match(/\n\t\.dialog \{[\s\S]*?\n\t\}/)?.[0];
				return rule ? /display:\s*flex/.test(rule) : false;
			})
			.map(([path]) => nameOf(path));

		expect(flexed).toEqual([]);
	});

	test('keeps the icon inside the heading so the box grows with the text', () => {
		// **繋ぎ忘れの見張り。**`header` の規則を消しただけで、
		// markup が `<header><Icon/><h2>` のままだと、**見た目は直りません。**
		const stragglers = dialogs
			.filter(([, source]) => /<header>\s*<Icon/.test(source))
			.map(([path]) => nameOf(path));

		expect(stragglers).toEqual([]);
	});
});

/**
 * **ホスト鍵の問いは、画面に 1 つだけ**（Issue #26 の後始末）。
 *
 * `+page.svelte` のコメントが、先に警告していました ——
 *
 * > 問いは `ConnectPanel` の中にしか無く、**MCP の経路から出す口がありませんでした。**
 * > 部品はそのまま使います —— **問いを 2 か所に作ると、片方だけ直る日が来ます**（D39）
 *
 * **#26 を直すとき、私はそれを読まずに 2 つ目を作りました。**
 * 結果、**人が［開く］で繋ぐと、帯とダイアログが両方出る**状態になりました。
 *
 * パスフレーズは 1 つです（`PassphraseDialog` を `ConnectPanel` も画面も使う）。
 * **ホスト鍵も同じにします。**
 */
describe('ホスト鍵の問い', () => {
	test('asks about host keys in exactly one place', () => {
		const asking = Object.entries(sources)
			.filter(([path]) => /\.svelte$/.test(path) && !/\.test\./.test(path))
			.filter(([, source]) => /files\.trust\.|hostkey\./.test(source))
			.map(([path]) => path.slice(path.lastIndexOf('/') + 1));

		// **問いの文言を持つのは `HostKeyDialog` だけ。**
		// `+page.svelte` は値を渡して描くだけなので、文言は持ちません。
		// **`ConnectPanel` が入っていたら、問いが 2 か所に在るということ。**
		expect(asking.sort()).toEqual(['HostKeyDialog.svelte']);
	});
});

/**
 * **端末の許可が通ったら、画面も端末へ動くこと**（実運用の指摘・2026-09-28）。
 *
 * > 端末をにぎりたいとアラートがでて、**通った時点で、一緒にタブも切り替わる**
 *
 * `viewAfterAnswered` を書いても、**`answerConsole` から呼ばなければ何も起きません。**
 * この試験は **#10 と同じ壊れ方**（作って、繋ぎ忘れる）だけを止めます。
 *
 * **中身が正しいかは見ていません。**見ているのは「繋がっているか」だけです。
 */
describe('端末の許可と画面', () => {
	test('**answerConsole が viewAfterAnswered を通している**', () => {
		const page = Object.entries(sources).find(([path]) => path.endsWith('routes/+page.svelte'));
		expect(page, '+page.svelte を読めていません').toBeTruthy();
		const text = page![1];
		const start = text.indexOf('async function answerConsole');
		expect(start, 'answerConsole が見つかりません').toBeGreaterThan(-1);
		// 関数の終わりまで（次の `\n\t}` まで）を見る。
		const body = text.slice(start, text.indexOf('\n\t}', start));
		expect(body).toContain('viewAfterAnswered');
	});
});

/**
 * **構造を持った失敗を、帯へ潰さない**（実運用の指摘・Issue #26）。
 *
 * > 上部に全幅で出る赤系の帯に **`[object Object]`** だけが出ています。
 *
 * 実行体は**構造を持ったまま**失敗を返します（画面が問いとして出せるように）。
 * それを `String(error)` で潰すと **`[object Object]`** になります。
 *
 * **中身が正しいかは見ていません。**見ているのは
 * 「**構造を持ちうる所で、素の `String(error)` を使っていないか**」だけです。
 */
describe('繋ぎ損ねた理由', () => {
	const page = () =>
		Object.entries(sources).find(([path]) => path.endsWith('routes/+page.svelte'))![1];

	/** **注記の中は数えない。**説明に書いた字を「使っている」と読むと、
	    直したのに落ちたままになります（実際そうなりました）。 */
	function withoutNotes(text: string): string {
		return text.replace(/\/\/.*$/gm, '').replace(/\/\*[\s\S]*?\*\//g, '');
	}

	test('**answerHostKey が String(error) で潰していない**', () => {
		const text = page();
		const start = text.indexOf('async function answerHostKey');
		expect(start, 'answerHostKey が見つかりません').toBeGreaterThan(-1);
		const body = withoutNotes(text.slice(start, text.indexOf('\n\t}', start)));
		expect(body).not.toContain('String(error)');
		expect(body).toContain('readableFailure');
	});

	test('**承認したあとパスフレーズが要るなら、その箱を開く**', () => {
		// **帯に落とさない。**落とすと、人は承認したのに先へ進めません。
		const text = page();
		const start = text.indexOf('async function answerHostKey');
		const body = text.slice(start, text.indexOf('\n\t}', start));
		expect(body).toContain('passphraseNeeded');
	});
});

/**
 * **繋ぐ道は、どの面にも同じだけ在ること**（実機の指摘・2026-10-04）。
 *
 * > **端末画面に、別の接続を開くがないです。**
 *
 * **そのとおりでした。**ファイルの面には「別の接続を開く」（`files.another`）が
 * 在り、端末の面には**在りませんでした。**端末の面に並ぶのは
 * **既に繋がっているもの**だけで、1 本繋いだ時点で `ConnectPanel` が消えます。
 * つまり **2 台目へ繋ぐには、いちどファイルの面か接続の面へ移るしかなかった。**
 *
 * これは Issue #9 と同じ壊れ方です ——
 * **「できるようになった後にだけ現れる案内」**で、探している人には見えない。
 *
 * **3 台同時に作業する人が、端末の面から出られない形**にしません。
 */
/** `+page.svelte` のソース。**glob の鍵が変わった日に黙って空振りしないこと。** */
function pageSource(): string {
	const found = Object.entries(sources).find(([path]) => path.endsWith('/routes/+page.svelte'));
	if (!found) throw new Error('+page.svelte が読めていません（glob の鍵が変わった？）');
	return found[1];
}

describe('繋ぐ道', () => {
	test('offers "open another connection" on the console screen too', () => {
		// **実機の指摘**（2026-10-04）—— 端末の面に「別の接続を開く」が無かった。
		//
		// **検査は `ConnectionRail.svelte.test.ts` へ移しました**（部品に出したので
		// 実際に押して確かめられます）。ここに残すのは繋がっているかだけ。
		const text = Object.entries(sources).find(([path]) =>
			path.endsWith('routes/+page.svelte')
		)![1];
		expect(text).toContain('<ConnectionRail');
		expect(text).toContain('onadd={() => (consoleAddingAnother = !consoleAddingAnother)}');
	});

	test('uses the same picker on both screens rather than a copy of it', () => {
		// **繋ぐ所を写して 2 か所に置くと、片方だけ直る日が来ます**（D39）。
		// 選ぶ部品は `ConnectPanel` 1 つで、面はそれを描くだけ。
		//
		// **「繋ぐ呼び出しが 1 か所」ではありません。**`+page.svelte` も
		// ホスト鍵とパスフレーズの答えから繋ぎ直します（問いは画面に 1 つ・D39）。
		// 見ているのは**選ぶ画面を自前で持っていないこと**です。
		const pickers = Object.entries(sources)
			.filter(([path]) => /\.svelte$/.test(path) && !/\.test\./.test(path))
			.filter(([, source]) => /connections_list/.test(source))
			.map(([path]) => path.slice(path.lastIndexOf('/') + 1));

		// `FileBrowser` も一覧を読みますが、**印の色をタブへ出すためだけ**です
		// （そう書いてあります）。`ConnectionManager` は登録の面。
		// **4 つ目が増えたら、それは写した疑いです。**
		expect(pickers.sort()).toEqual([
			'ConnectPanel.svelte',
			'ConnectionManager.svelte',
			'FileBrowser.svelte'
		]);
	});
});

/**
 * **構造を持った失敗を `String()` で潰さない**（Issue #33 / #26）。
 *
 * `session_connect` は**構造を持った失敗**を返します
 * （ホスト鍵・パスフレーズ・その他）。`String(error)` だと
 * **赤帯が `[object Object]` になり、人にも私たちにも何も伝わりません。**
 *
 * **2 回同じことが起きています。**#26 で 1 か所直し、
 * **#33 で残りの 2 か所が見つかりました**（パスフレーズを答えた後と `ConnectPanel`）。
 * 「この字だけで 2 日使った」と書かれています。**3 回目を作りません。**
 */
describe('繋ぐ失敗の見せ方', () => {
	test('every place that connects makes the failure readable', () => {
		const connecting = Object.entries(sources).filter(
			([path, source]) =>
				/\.svelte$/.test(path) && !/\.test\./.test(path) && /session_connect/.test(source)
		);

		// **この検査自体が空振りしていないこと。**
		expect(connecting.length).toBeGreaterThanOrEqual(2);

		const unreadable = connecting
			.filter(([, source]) => !/readableFailure/.test(source))
			.map(([path]) => path.slice(path.lastIndexOf('/') + 1));

		expect(
			unreadable,
			`session_connect を呼ぶのに readableFailure を使っていない: ${unreadable.join(', ')}`
		).toEqual([]);
	});

	test('the readable failure never leaves an object as the text', () => {
		// **最後の砦が在ること。**`[object Object]` になる経路を 1 つも残さない。
		const made = sources['../lib/connect-failure.ts'] ?? pickConnectFailure();
		expect(made).toContain('JSON.stringify');
	});
});

/** `connect-failure.ts` のソース。**glob の鍵が変わった日に空振りしないこと。** */
function pickConnectFailure(): string {
	const found = Object.entries(sources).find(([path]) => path.endsWith('/connect-failure.ts'));
	if (!found) throw new Error('connect-failure.ts が読めていません');
	return found[1];
}

/**
 * **`~/.ssh/config` からの取り込み**（UI 案・4 枚目）。
 *
 * ここで守るのは 3 つです。
 *
 * 1. **AI にこの口を出さない。**あのファイルは**接続先そのものの一覧**で、
 *    人が繋いでいない所まで全部並んでいます（禁止事項 4 / D11）。
 * 2. **書かない。**ssh 本体と他の道具が共有しているので、
 *    **こちらの書式の思い込みで壊すと、人は ssh そのものを失います。**
 * 3. **柵は空のまま足す**（D22 —— 既定は空＝AI は読むだけ）。
 */
/**
 * 関数 1 本の**本体だけ**を切り出す（閉じカッコまで）。
 *
 * **文字数で切らないこと。**次の関数や、間に挟んだ説明文の中の名前を拾います。
 */
function bodyOf(text: string, signature: string): string {
	const at = text.indexOf(signature);
	if (at === -1) throw new Error(`${signature} が見つかりません`);
	// 字下げ 1 段の関数なので、閉じカッコは `\n\t}` で終わります。
	const end = text.indexOf('\n\t}', at);
	if (end === -1) throw new Error(`${signature} の終わりが見つかりません`);
	return text.slice(at, end);
}

describe('~/.ssh/config からの取り込み', () => {
	const manager = () =>
		Object.entries(sources).find(([path]) =>
			path.endsWith('components/ConnectionManager.svelte')
		)![1];

	test('is offered to the person, in words', () => {
		expect(manager()).toContain("i18n.t('conn.import')");
		expect(manager()).toContain("invoke<Candidate[]>('ssh_config_candidates')");
	});

	test('does not register anything by itself', () => {
		// **そのままでは登録しません。**人が 1 件ずつ押して足します。
		//
		// **関数の本体だけを見ること。**2 回つまずきました（2026-10-23）——
		// 文字数で切ると次の関数の `connection_save` を拾い、
		// 次の関数まで切ると**間に挟んだ説明文の中の名前**を拾いました。
		expect(bodyOf(manager(), 'async function readSshConfig')).not.toContain('connection_save');
	});

	test('adds with an empty fence', () => {
		// **既定は空＝AI は読むだけ**（D22）。取り込みで柵を開けない。
		expect(manager()).toContain('write_roots: []');
	});

	test('fills in the default port here, not in Rust', () => {
		// Rust 側が `null` で返すのは、**「書いてあった 22」と「書いていない」を
		// 区別する**ため。既定を当てるのは呼ぶ側。
		expect(manager()).toContain('port: one.port ?? 22');
	});

	test('lets the place be hidden', () => {
		// **接続先そのもの**（禁止事項 4）。
		expect(manager()).toContain('<span class="import-where" data-secret>');
	});

	test('does not swallow a read failure', () => {
		// **読めなかったのと 0 件は別です。**
		const text = manager();
		expect(bodyOf(text, 'async function readSshConfig')).toContain('failure = String(error)');
	});

	test('does not quietly overwrite a connection that is already registered', () => {
		// **`connection_save` は同じ識別子を上書きします。**
		// `~/.ssh/config` に同じ名前が在ると、人が並べた柵（`write_roots`）ごと
		// 消えます ——「取り込んだら AI が書けなくなった」を作らない。
		//
		// **押せる所に出さない**のが答えです（出して断るのでは、人は理由を探します）。
		const text = manager();
		expect(text).toContain('alreadyHere');
		const at = text.indexOf('function alreadyHere');
		expect(at).toBeGreaterThan(-1);
		expect(text.slice(at, at + 260)).toContain('items.some');
	});

	test('says so instead of hiding the collision', () => {
		// **黙って消すのも、黙って出さないのも、どちらも読めません。**
		expect(manager()).toContain("i18n.t('conn.import.here')");
	});

	// **「AI に出していない」はここでは見張れません**（2026-10-23 に空振り）。
	//
	// `import.meta.glob('../**/*.{svelte,ts}')` は `apps/desktop/src` だけを拾い、
	// **Rust を 1 件も見ていませんでした**（鍵 114 件・`sshboard-mcp` は 0 件）。
	// 0 件の配列を回るので、**見張っていないのに緑**でした。
	//
	// 見張りは `crates/sshboard-mcp/tests/mcp_over_http.rs` に移しました ——
	// **あちらは本物の道具一覧を見ます。**
});

describe('AI が走らせられることの一覧', () => {
	/**
	 * **人だけが見られませんでした**（2026-10-23）。
	 *
	 * 実行体は両方のファイルを読めて、AI も `list_operations` で読めるのに、
	 * **画面へ出す口が 1 つもありませんでした。**
	 * 本番で詰まったのはここで、人は書く場所を探して見つけられませんでした。
	 */
	test('is reachable from the menu', () => {
		// **開けない面は無いのと同じ。**
		const text = pageSource();
		expect(text).toContain('allowlistOpen = true');
		expect(text).toContain('<AllowlistDialog');
	});

	test('asks the engine for both lists', () => {
		expect(pageSource()).toContain("invoke<AllowlistOverview>('allowlist_overview')");
	});

	test('runs nothing from this screen', () => {
		// **釦を置くと、「人が押した」と「AI が頼んだ」が混ざります。**
		// 走らせるのは承認の面（`OperationRequestDialog`）です。
		const dialog = components['./components/AllowlistDialog.svelte'];
		expect(dialog, 'AllowlistDialog.svelte が無い').toBeTruthy();
		expect(dialog).not.toContain('invoke');
		expect(dialog).not.toContain('run_operation');
	});

	test('hides what would be read off a shared screen', () => {
		// **コマンドにはサーバー側の道とドメインが入ります**（禁止事項 4）。
		const dialog = components['./components/AllowlistDialog.svelte'];
		expect(dialog).toContain('class="run" data-secret');
		expect(dialog).toContain('<code data-secret>{group.list.path}</code>');
	});
});

describe('AI に渡す（案・1 枚目）', () => {
	/**
	 * **人の側から渡せること。**
	 *
	 * それまで握りが AI へ移る道は 1 本だけでした —— AI が頼み、人が許す。
	 * 実機の指摘（2026-09-06）——
	 * 「AI に渡すときに［止める］を押さないといけません」。
	 */
	test('is offered while the human is holding it', () => {
		const text = pageSource();
		expect(text).toContain("invoke('console_give')");
		expect(text).toContain("i18n.t('console.give')");
	});

	test('is not offered when the human is not holding it', () => {
		// **押せない釦を置かない。**握っていないのに「渡す」は意味を成しません。
		// 渡す釦は `iHold` の枝の中にあること。
		const text = pageSource();
		const at = text.indexOf("i18n.t('console.give')");
		expect(at).toBeGreaterThan(-1);
		// 直前の分岐を見る。**`iHold` を見ている枝の中**に居ること。
		const before = text.slice(Math.max(0, at - 700), at);
		expect(before).toMatch(/\{:else if\s+holder\s+&&\s+iHold\}/);
	});

	test('does not ask the human again', () => {
		// **人が渡しているのに問いを出さない。**
		// 実行体側（`console_give`）が `ask_first` を通さないことは Rust の試験で見ています。
		// ここでは、画面が承認の道（`console_answer`）を経由していないことを見ます。
		const text = pageSource();
		const at = text.indexOf('async function giveConsole');
		const end = text.indexOf('\n\t}', at);
		expect(at).toBeGreaterThan(-1);
		expect(text.slice(at, end)).not.toContain('console_answer');
	});
});

describe('帯の書き出し（案・4 枚目）', () => {
	/**
	 * **囲い（D22）は落とし先を守りません。**
	 * D27 で「手元へ落とす口は AI に作らない」と決めたのと同じ性質です。
	 */
	test('is reachable from the menu', () => {
		const text = pageSource();
		expect(text).toContain('bandExportOpen = true');
		expect(text).toContain('<BandExportDialog');
	});

	test('shows what goes in the file before writing it', () => {
		// **押してから知るのでは遅い。**案の書き出し側と同じ形（含める / 含めない）。
		const dialog = components['./components/BandExportDialog.svelte'];
		expect(dialog, 'BandExportDialog.svelte が無い').toBeTruthy();
		for (const key of ['band.export.in.who', 'band.export.in.secret', 'band.export.warn']) {
			expect(dialog.includes(key), `${key} が出ていません`).toBe(true);
		}
	});

	test('lets the human pick where it lands', () => {
		// **落とし先は人が選びます。**黙ってどこかへ書きません。
		const text = pageSource();
		const at = text.indexOf('async function saveBand');
		const end = text.indexOf('\n\t}', at);
		expect(at).toBeGreaterThan(-1);
		expect(text.slice(at, end)).toContain('saveDialog');
	});

	test('follows the screen when it is hidden', () => {
		// **見えているものと、書き出したものが違う、を作りません。**
		expect(pageSource()).toContain('masked={redaction.on}');
	});
});

describe('端末が 1 枚も無いとき（実機の報告・2026-10-23）', () => {
	/**
	 * **「何の反応も無い」の正体。**
	 *
	 * 実機で `doko005` を MCP の disconnect で切ったあと、
	 * **黒い面がそのまま残りました。**
	 *
	 * 残っていたのではありません —— **言葉が 1 つも無い空の面**が
	 * 描かれていました（`<div class="core terminal-core empty"></div>`）。
	 * 人からは「切ったのに何も起きていない」と読めます。
	 *
	 * **止まっている理由が人へ届かないと、人は製品の外で直します。**
	 * 今日の本番で起きたのと同じ形です。
	 */
	test('says why nothing is there instead of showing a blank black panel', () => {
		const text = pageSource();
		// **空の面を言葉なしで描かないこと。**
		expect(text).not.toContain('<div class="core terminal-core empty"></div>');
		expect(text).toContain("i18n.t('console.none.connected')");
		expect(text).toContain("i18n.t('console.none.disconnected')");
	});

	test('tells the human what to press next', () => {
		// 「無い」だけでは、次に何をすればいいか分かりません。
		const text = pageSource();
		const at = text.indexOf("i18n.t('console.none.disconnected')");
		expect(at).toBeGreaterThan(-1);
	});
});

describe('問いに気づけるように（Issue #30）', () => {
	/**
	 * > パス入れるときに、アプリを前面に出せます？
	 * > **私の場合、裏で聞かれていて気づかないですね**
	 *
	 * **既定は点滅だけ。**前面強制は選んだ人だけです ——
	 * 報告者自身が「**人によっては邪魔**」と書いています。
	 */
	test('lets the human choose, from the menu', () => {
		// **設定ファイルを探させない。**押せる所に置きます。
		const text = pageSource();
		expect(text).toContain("i18n.t('attention.label')");
		expect(text).toContain("invoke('attention_set'");
	});

	test('offers flashing first, not forcing to the front', () => {
		// **並びが既定を語ります。**前面強制を先頭に置くと、人はそれを選びます。
		const text = pageSource();
		const at = text.indexOf("i18n.t('attention.label')");
		const row = text.slice(at, at + 700);
		expect(row.indexOf("'flash'")).toBeGreaterThan(-1);
		expect(row.indexOf("'flash'")).toBeLessThan(row.indexOf("'front'"));
	});

	test('reads the current choice at startup', () => {
		// 読まないと、**選んだはずの設定が画面に出ません。**
		expect(pageSource()).toContain("invoke<AttentionHow>('attention_how')");
	});

	test('says what each choice does', () => {
		// 「前へ出す」だけでは、割り込むことが伝わりません。
		expect(pageSource()).toContain('attention.${how}.what');
	});
});
