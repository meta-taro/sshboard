/**
 * AI が「こちらを見てください」と言ってきたとき、**従ってよいか**（D44 / Issue #15）。
 *
 * 実機で起きた並びがこれでした。
 *
 * 1. AI が端末を開く → 承認ダイアログが**［接続］タブの上**に出る
 * 2. 人が［許可する］を押す
 * 3. AI が端末で打ち、出力を受け取る
 * 4. **人は［接続］タブを見たままで、［端末］で何が起きているか分からない**
 *
 * > **AI 側には「こちらを見てください」と言う手段がありません。**
 *
 * そこで口を開けます。ただし **AI に画面を奪わせません。**
 * ここが決めるのは「切り替えるかどうか」だけで、**承認も実行も増えません。**
 */
import { describe, expect, test } from 'vitest';

import { canFollow, HUMAN_HOLDS_MS, isView, VIEWS, viewAfterAnswered } from './view-request';

/** 何も邪魔していない状態。**ここから 1 つずつ崩して確かめます。** */
const calm = {
	askOpen: false,
	focusInField: false,
	sinceHumanSwitchMs: HUMAN_HOLDS_MS + 1
};

describe('isView', () => {
	test('takes only the names the screen actually has', () => {
		// **呼び名を 1 つにする**（D44）。`terminal` ではなく `console`。
		for (const name of VIEWS) expect(isView(name)).toBe(true);
		for (const wrong of ['terminal', 'activity', 'log', '', 'CONSOLE'])
			expect(isView(wrong), `${wrong} を通してはいけない`).toBe(false);
	});
});

describe('canFollow', () => {
	test('follows when nothing is in the way', () => {
		expect(canFollow(calm)).toBe(true);
	});

	test('never moves the screen out from under an open question', () => {
		// パスフレーズや端末の許可を聞いている最中に画面が動くと、
		// **人は何に答えたのか分からなくなります。**
		expect(canFollow({ ...calm, askOpen: true })).toBe(false);
	});

	test('never moves the screen while the person is typing', () => {
		// 入力欄に焦点があるのに切り替えると、**打っていたものが消えます。**
		expect(canFollow({ ...calm, focusInField: true })).toBe(false);
	});

	test('leaves alone what the person just chose', () => {
		// **人が選んだ画面を、AI が上書きし続ける形にしない。**
		expect(canFollow({ ...calm, sinceHumanSwitchMs: 0 })).toBe(false);
		expect(canFollow({ ...calm, sinceHumanSwitchMs: HUMAN_HOLDS_MS - 1 })).toBe(false);
	});

	test('follows again once the person has moved on', () => {
		expect(canFollow({ ...calm, sinceHumanSwitchMs: HUMAN_HOLDS_MS })).toBe(true);
	});

	test('holds for long enough to matter, but not so long it never helps', () => {
		// **短すぎると奪ったのと同じ。長すぎると口が無いのと同じ。**
		expect(HUMAN_HOLDS_MS).toBeGreaterThanOrEqual(3000);
		expect(HUMAN_HOLDS_MS).toBeLessThanOrEqual(30000);
	});
});

/**
 * **許可したら、その面へ動く**（実運用の指摘・2026-09-28）。
 *
 * > 端末をにぎりたいとアラートがでて、**通った時点で、一緒にタブも切り替わる**
 *
 * **AI が `show_view` を別に呼ぶのを当てにしない。**呼び忘れたら人は見ないまま、
 * 呼んでも `canFollow` で止まりうる。**許可したのは端末なので、
 * AI の裁量に委ねる場面ではありません。**
 */
describe('答えたあとの面', () => {
	test('**端末を許可したら、端末の面へ**', () => {
		expect(viewAfterAnswered('console', true)).toBe('console');
	});

	test('断ったときは動かさない', () => {
		// **断った人を端末へ連れて行くのは、押し売りです。**
		expect(viewAfterAnswered('console', false)).toBeNull();
	});
});
