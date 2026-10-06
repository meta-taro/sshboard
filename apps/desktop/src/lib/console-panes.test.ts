/**
 * **端末を 2 枚に割る規則**（DESIGN.md「端末の割り方」・2026-10-06）。
 *
 * 人が決めた形 ——
 *
 * > 見た目はデスクトップみたいに半々とかタブ切り替えかなぁ VSCode もそんな感じだったような
 * > 番号とかタイトルとかいろとかできると尚良い w
 *
 * **ここが見るのは「どの端末がどちらの面に出るか」だけ**です。
 * 描く所（xterm）とは分けてあります —— 規則だけなら、画面を描かずに確かめられます。
 */
import { describe, expect, test } from 'vitest';

import { paneLayout, type ConsolePane } from './console-panes';

const pane = (id: number, connection: string, holder: 'human' | 'ai' = 'human'): ConsolePane => ({
	id,
	connection,
	holder
});

describe('端末の割り振り', () => {
	test('shows nothing when no console is open', () => {
		// **空を「壊れている」と見せない。**開いていないだけ。
		const shown = paneLayout([], null, null);
		expect(shown.panes).toEqual([]);
		expect(shown.focused).toBeNull();
	});

	test('puts the only console in the first pane', () => {
		const shown = paneLayout([pane(1, 'web')], 'web', null);
		expect(shown.panes.map((p) => p.id)).toEqual([1]);
		expect(shown.focused).toBe(1);
	});

	test('splits two consoles on the same server side by side', () => {
		// **これが目的です。**同じサーバに 2 枚 ——
		// 「同じサーバに２画面入る場合もある」
		const shown = paneLayout([pane(1, 'web'), pane(2, 'web')], 'web', null);
		expect(shown.panes.map((p) => p.id)).toEqual([1, 2]);
	});

	test('only shows the consoles of the connection being looked at', () => {
		// **別のサーバの面を混ぜない。**人は宛先のタブを見ています。
		const shown = paneLayout([pane(1, 'web'), pane(2, 'db')], 'web', null);
		expect(shown.panes.map((p) => p.id)).toEqual([1]);
	});

	test('keeps at most two panes and leaves the rest to the tabs', () => {
		// **3 分割はしません**（狭い画面で読めない・DESIGN.md）。
		// 3 枚目はタブで切り替えます。
		const all = [pane(1, 'web'), pane(2, 'web'), pane(3, 'web')];
		const shown = paneLayout(all, 'web', null);
		expect(shown.panes.length).toBeLessThanOrEqual(2);
		expect(shown.spare.map((p) => p.id)).toEqual([3]);
	});

	test('a chosen console is always one of the panes', () => {
		// **選んだものが出ないのは、押せない釦と同じ。**
		const all = [pane(1, 'web'), pane(2, 'web'), pane(3, 'web')];
		const shown = paneLayout(all, 'web', 3);
		expect(shown.panes.map((p) => p.id)).toContain(3);
		expect(shown.focused).toBe(3);
	});

	test('forgets a choice that is no longer open', () => {
		// **閉じた番号を選んだままにしない。**番号は使い回されないので、
		// 「消えた」以外の意味になりません。
		const shown = paneLayout([pane(1, 'web')], 'web', 9);
		expect(shown.focused).toBe(1);
		expect(shown.panes.map((p) => p.id)).toEqual([1]);
	});

	test('orders panes by number so they do not dance', () => {
		// **並びが毎回変わると、人は左右を取り違えます。**
		const shown = paneLayout([pane(2, 'web'), pane(1, 'web')], 'web', null);
		expect(shown.panes.map((p) => p.id)).toEqual([1, 2]);
	});

	test('says whether another one can be opened', () => {
		expect(paneLayout([], 'web', null).canOpenAnother).toBe(true);
		expect(paneLayout([pane(1, 'web')], 'web', null).canOpenAnother).toBe(true);
		// **上限は実行体が決めます**（PER_CONNECTION_LIMIT）。
		// 画面は、押しても断られる釦を出しません。
		expect(paneLayout([pane(1, 'web'), pane(2, 'web')], 'web', null).canOpenAnother).toBe(false);
	});

	test('does not offer another one when nothing is connected', () => {
		expect(paneLayout([], null, null).canOpenAnother).toBe(false);
	});
});
