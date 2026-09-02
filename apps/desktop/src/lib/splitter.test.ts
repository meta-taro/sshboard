import { beforeEach, describe, expect, test } from 'vitest';

/**
 * `localStorage` の最小実装。
 *
 * このワークスペースの vitest は node 環境で走ります（jsdom を入れていません）。
 * **jsdom を足すためだけに依存を増やしません** — ここで確かめたいのは
 * 丸めと保存のロジックであって、ブラウザの実装ではないためです。
 */
class MemoryStorage {
	private data = new Map<string, string>();
	getItem(key: string): string | null {
		return this.data.get(key) ?? null;
	}
	setItem(key: string, value: string): void {
		this.data.set(key, String(value));
	}
	removeItem(key: string): void {
		this.data.delete(key);
	}
	clear(): void {
		this.data.clear();
	}
}

globalThis.localStorage = new MemoryStorage() as unknown as Storage;

import {
	clampListWidth,
	clampPaneRatio,
	DEFAULT_LIST_WIDTH,
	DEFAULT_PANE_RATIO,
	loadListWidth,
	loadPaneRatio,
	MIN_LIST_WIDTH,
	MIN_PANE_RATIO,
	savePaneRatio
} from './splitter.svelte';

beforeEach(() => {
	localStorage.clear();
});

describe('clampListWidth — 固定幅の側（接続管理）', () => {
	test('枠に収まる値はそのまま通す', () => {
		expect(clampListWidth(300, 1000)).toBe(300);
	});

	test('名前が読めなくなる幅まで縮めさせない', () => {
		expect(clampListWidth(10, 1000)).toBe(MIN_LIST_WIDTH);
	});

	test('入力側が潰れる幅まで広げさせない', () => {
		// 上限は枠に対する割合。**固定ピクセルにすると窓を狭めたとき入力側が消える。**
		expect(clampListWidth(900, 1000)).toBe(600);
	});

	test('枠が極端に狭くても、最小幅は下回らない', () => {
		expect(clampListWidth(500, 100)).toBe(MIN_LIST_WIDTH);
	});
});

describe('clampPaneRatio — 割合の側（ファイル 2 ペイン）', () => {
	test('真ん中はそのまま', () => {
		expect(clampPaneRatio(0.5)).toBe(0.5);
	});

	test('片側が読めなくなるまで寄せさせない（左）', () => {
		expect(clampPaneRatio(0.01)).toBe(MIN_PANE_RATIO);
	});

	test('片側が読めなくなるまで寄せさせない（右）', () => {
		expect(clampPaneRatio(0.99)).toBe(1 - MIN_PANE_RATIO);
	});

	test('数でないものが来たら定位置へ倒す', () => {
		// **落とさない。**割合が壊れて画面が消える方が困る。
		expect(clampPaneRatio(Number.NaN)).toBe(DEFAULT_PANE_RATIO);
		expect(clampPaneRatio(Number.POSITIVE_INFINITY)).toBe(DEFAULT_PANE_RATIO);
	});
});

describe('割合の保存と読み出し', () => {
	test('何も保存されていなければ真ん中', () => {
		expect(loadPaneRatio()).toBe(DEFAULT_PANE_RATIO);
	});

	test('保存した割合が戻る', () => {
		savePaneRatio(0.32);
		expect(loadPaneRatio()).toBeCloseTo(0.32, 5);
	});

	test('壊れた値が入っていても定位置へ倒す', () => {
		localStorage.setItem('sshboard-pane-ratio', 'ずれた文字列');
		expect(loadPaneRatio()).toBe(DEFAULT_PANE_RATIO);
	});

	test('範囲外の値が入っていても丸めて返す', () => {
		localStorage.setItem('sshboard-pane-ratio', '0.98');
		expect(loadPaneRatio()).toBe(1 - MIN_PANE_RATIO);
	});

	test('保存する側でも丸める。**壊れた値を残さない**', () => {
		savePaneRatio(5);
		expect(loadPaneRatio()).toBe(1 - MIN_PANE_RATIO);
	});
});

describe('既存の値との地続き', () => {
	test('保存が無ければ既定幅', () => {
		expect(loadListWidth()).toBe(DEFAULT_LIST_WIDTH);
	});
});
