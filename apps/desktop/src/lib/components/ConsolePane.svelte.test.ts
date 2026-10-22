/**
 * **端末の面を、描いて確かめる**（Issue #10 の再発を止める）。
 *
 * 端末タブは 2026-09-01 に入って以来、**3 日間 1 バイトも表示していませんでした。**
 * 原因は「作った端末へ書く行がどこにも無い」という 1 行です。
 * **型検査は 283 files 0 errors のまま**で、CI も緑でした。
 *
 * `terminal-wiring.test.ts` がソースの形だけを見て同じ穴を塞いでいますが、
 * **実際に描いて確かめるものは 1 本もありませんでした。**
 * 1 枚分を部品に切り出したので、ここから書けます。
 *
 * **xterm そのものは偽ります。**jsdom に canvas がなく、
 * ここで見たいのは**配線**です —— 作った端に書き戻すか、打鍵がどの番号へ行くか。
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

/** 作られた端末 1 つぶん。 */
interface FakeTerm {
	written: number[][];
	onData: Array<(data: string) => void>;
	options: { fontSize: number };
	element: HTMLElement;
}

const made: FakeTerm[] = [];

vi.mock('$lib/terminal.svelte', () => ({
	createTerminal: (host: HTMLElement, fontSize: number) => {
		const element = document.createElement('div');
		host.appendChild(element);
		const term: FakeTerm = { written: [], onData: [], options: { fontSize }, element };
		made.push(term);
		return {
			element,
			options: term.options,
			onData: (fn: (data: string) => void) => term.onData.push(fn),
			onResize: () => {},
			dispose: () => {},
			// **どの端末へ書いたかを覚えます。**作ったのに書かない、を捕まえるため。
			__write: (payload: number[]) => term.written.push([...payload])
		};
	},
	writeChunk: (term: { __write?: (payload: number[]) => void }, payload: number[]) => {
		term.__write?.(payload);
	},
	attachFit: () => () => {},
	attachSearch: () => ({ open: () => {}, close: () => {}, isOpen: () => false })
}));

vi.mock('$lib/terminal-clipboard', () => ({
	attachClipboard: () => () => {}
}));

import { i18n } from '$lib/i18n/i18n.svelte';

import ConsolePane from './ConsolePane.svelte';

/** **合成値。**実物の接続先はテストにも置きません（PRD §8）。 */
const clipboard = {
	writeText: async () => {},
	readText: async () => '',
	onError: () => {}
};

beforeEach(() => {
	invoke.mockReset();
	invoke.mockResolvedValue(undefined);
	made.length = 0;
});

afterEach(cleanup);

function mount(over: Record<string, unknown> = {}) {
	return render(ConsolePane, {
		props: {
			pane: { id: 2, connection: 'web-prod', holder: 'human' as const },
			clipboard,
			platform: 'mac' as const,
			iHold: true,
			...over
		}
	});
}

describe('端末 1 枚の面', () => {
	test('shows the number, the connection and who holds it', () => {
		// **番号を出すこと。**人が口で「②を見て」と言えて、AI も同じ番号で指せる
		// （PRD §4-0 の「同じ視点」）。
		mount();

		expect(screen.getByText('#2')).toBeTruthy();
		expect(screen.getByText('web-prod')).toBeTruthy();
		// **文言は `console.held.*` から。**
		// 以前はここに `'人'` / `'AI'` が**日本語のまま直書き**されていました
		// （画面は 11 言語あります）。訳の在る鍵に寄せました（UI 案・2026-10-21）。
		expect(screen.getByText(i18n.t('console.held.me'))).toBeTruthy();
	});

	test('says when the AI is the one holding it', () => {
		// **見えない所で AI が打っている、を作らない**（D29）。
		mount({ pane: { id: 1, connection: 'db-prod', holder: 'ai' as const } });

		expect(screen.getByText(i18n.t('console.held.ai'))).toBeTruthy();
	});

	test('offers a stop button on the pane the AI is holding', () => {
		// **人はいつでも取り返せる**（D29）。約束が**その面の上に在る**こと ——
		// 別の所に在ると、2 枚開いた日に「どちらを止めるのか」が言えません。
		const stopped: number[] = [];
		mount({
			pane: { id: 3, connection: 'db-prod', holder: 'ai' as const },
			onstop: (id: number) => stopped.push(id)
		});

		const stop = screen.getByRole('button', { name: i18n.t('console.stop') });
		stop.click();
		// **番号で止める。**接続の名前だと、同じサーバに 2 枚ある日に
		// どちらが止まったか言えません。
		expect(stopped).toEqual([3]);
	});

	test('does not offer it on a pane the person is holding', () => {
		// 自分の面に［止める］を出すと、**何が止まるのか読めません。**
		//
		// **`onstop` を渡すこと。**最初は渡していなくて、ボタンが出ない理由が
		// 「人の面だから」ではなく**「口が無いから」**でした。
		// それだと、人の面にも出す壊し方が素通りします（実際に素通りしました）。
		mount({ onstop: () => {} });
		expect(screen.queryByRole('button', { name: i18n.t('console.stop') })).toBeNull();
	});

	test('writes what it remembered into the terminal it just made', () => {
		// **これが Issue #10 の中身です。**作ったのに書かない端末を作らない。
		mount({ backlog: [65, 66, 67] });

		expect(made).toHaveLength(1);
		expect(made[0].written).toEqual([[65, 66, 67]]);
	});

	test('writes what arrives while it is open, not just at mount', () => {
		// **2026-10-07 の回帰。**0.1.21 で部品へ切り出したとき、
		// **流れてくる分を書く経路が落ちていました。**
		//
		// > リアルタイムでコマンドを打てたのが分からず、画面を切り替えないと反映されない
		//
		// **AI が `rm` を打っても、人の画面は静かなまま**でした ——
		// 「人が常に見ている」（D29）が崩れます。
		//
		// **「作った端に書き戻す」試験は通っていました。**あれは**マウント時の 1 回**で、
		// **開いたまま流れてくる**経路とは別物です。**別物なので、別に見張ります。**
		const writers: Array<(chunk: number[]) => void> = [];
		mount({
			backlog: [65],
			onready: (_id: number, write?: (chunk: number[]) => void) => {
				if (write) writers.push(write);
			}
		});

		expect(writers, '流れてくる分を書く口を親へ渡していない').toHaveLength(1);

		writers[0]([66, 67]);
		// 1 回目が控え、2 回目が流れてきた分。**両方描かれていること。**
		expect(made[0].written).toEqual([[65], [66, 67]]);
	});

	test('takes the write handle back when the terminal goes away', () => {
		// **閉じた面へ書き続けない。**捨てた端末へ書くと、
		// 「出ているはずなのに見えない」が起きます。
		const seen: Array<[number, unknown]> = [];
		const { unmount } = mount({
			onready: (id: number, write: unknown) => seen.push([id, write])
		});

		expect(seen[0][1]).toBeTruthy();
		unmount();
	});

	test('sends what is typed to its own number', () => {
		// **番号で送ること。**接続の名前で送ると、同じサーバに 2 枚開いた日に
		// **どちらへ打ったか言えません。**
		mount();
		made[0].onData.forEach((fn) => fn('ls\n'));

		expect(invoke).toHaveBeenCalledWith('console_type_into', {
			consoleId: 2,
			bytes: [...new TextEncoder().encode('ls\n')]
		});
	});

	test('types nothing when the person is not holding it', () => {
		// **往復させて断られるより、画面で止める方が速い**（実行体も同じ判断をします）。
		mount({ iHold: false });
		made[0].onData.forEach((fn) => fn('rm -rf /\n'));

		expect(invoke).not.toHaveBeenCalled();
	});

	test('tells the parent when it is picked', async () => {
		// **触った面へ打鍵が行く。**どちらに打っているか分からないのが、いちばん危ない。
		const picked: number[] = [];
		const { container } = mount({ onpick: (id: number) => picked.push(id) });

		const screenEl = container.querySelector('.screen');
		expect(screenEl).toBeTruthy();
		await fireEvent.mouseDown(screenEl!);
		expect(picked).toEqual([2]);
	});

	test('hands the search handle up with its number', () => {
		// **検索の窓は 1 つで、面は 2 枚。**どの面を探すのかは親が決めます。
		const handles: Array<[number, unknown]> = [];
		mount({ onsearch: (id: number, search: unknown) => handles.push([id, search]) });

		expect(handles).toHaveLength(1);
		expect(handles[0][0]).toBe(2);
		expect(handles[0][1]).toBeTruthy();
	});

	test('keeps the font size in step with the rest of the app', () => {
		// **xterm は自前で描くので `rem` が効きません。**
		// 画面だけ大きくなって端末が小さいままだと、同じ 1 つの道具に見えません。
		mount({ fontPx: 18 });

		expect(made[0].options.fontSize).toBe(18);
	});

	test('shows the colour the person gave the connection', () => {
		// **端末のための色をもう 1 つ作らない**（DESIGN.md）——
		// 接続の一覧とファイルの面と**同じ札**を使います。
		// 色が別物だと、人は「どれが本番か」を 2 回覚えることになります。
		const { container } = mount({ mark: 'red' });
		const pane = container.querySelector('.pane') as HTMLElement;

		expect(pane.style.getPropertyValue('--mark')).toBe('var(--mark-red)');
		expect(container.querySelector('.mark-bar')).toBeTruthy();
	});

	test('leaves the colour slot empty rather than guessing one', () => {
		// **印を付けていない接続に色を作りません。**
		// 勝手に振ると、人が付けた色と意味が混ざります。
		// **場所は残します** —— 無くすと、色のある面と並びがずれます。
		const { container } = mount({ mark: null });
		const pane = container.querySelector('.pane') as HTMLElement;

		expect(pane.style.getPropertyValue('--mark')).toBe('transparent');
		expect(container.querySelector('.mark-bar')).toBeTruthy();
	});

	test('marks the pane the keystrokes go to', () => {
		const { container } = mount({ active: true });
		expect(container.querySelector('.pane.active')).toBeTruthy();
	});

	test('does not mark a pane that is not the one being typed into', () => {
		const { container } = mount({ active: false });
		expect(container.querySelector('.pane.active')).toBeNull();
	});
});
