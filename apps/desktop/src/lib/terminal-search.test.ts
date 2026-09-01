/**
 * 端末の検索。**画面を開かなくても確かめられる部分。**
 *
 * ここも「触ってはいけない組み合わせ」が主役です。
 * 端末の `Ctrl+F` は **1 文字進む**（emacs 流のキー割り当て）。
 * 検索に取ると、**行の中を動けなくなります。**
 */
import { describe, expect, test, vi } from 'vitest';

import {
	createSearch,
	isFindShortcut,
	type SearchAddonLike,
	type SearchShortcutEvent
} from './terminal-search';

function keydown(part: Partial<SearchShortcutEvent>): SearchShortcutEvent {
	return { type: 'keydown', key: 'a', ctrlKey: false, metaKey: false, shiftKey: false, ...part };
}

describe('isFindShortcut', () => {
	test('takes Command+F on macOS', () => {
		expect(isFindShortcut(keydown({ key: 'f', metaKey: true }), 'mac')).toBe(true);
	});

	test('takes Ctrl+Shift+F where there is no Command key', () => {
		expect(isFindShortcut(keydown({ key: 'F', ctrlKey: true, shiftKey: true }), 'other')).toBe(
			true
		);
	});

	test('never treats a bare Ctrl+F as find', () => {
		// **端末の Ctrl+F は「1 文字進む」。**取ると行の中を動けなくなります。
		const event = keydown({ key: 'f', ctrlKey: true });

		expect(isFindShortcut(event, 'mac')).toBe(false);
		expect(isFindShortcut(event, 'other')).toBe(false);
	});

	test('ignores key releases', () => {
		const released = { ...keydown({ key: 'f', metaKey: true }), type: 'keyup' };

		expect(isFindShortcut(released, 'mac')).toBe(false);
	});
});

function fakeAddon(): SearchAddonLike & { calls: Array<[string, string]> } {
	const calls: Array<[string, string]> = [];
	return {
		calls,
		findNext: (term: string) => {
			calls.push(['next', term]);
			return term !== '';
		},
		findPrevious: (term: string) => {
			calls.push(['prev', term]);
			return term !== '';
		},
		clearDecorations: () => {
			calls.push(['clear', '']);
		}
	};
}

describe('createSearch', () => {
	test('searches forward for what was typed', () => {
		const addon = fakeAddon();
		const search = createSearch(addon);

		expect(search.next('error')).toBe(true);
		expect(addon.calls).toEqual([['next', 'error']]);
	});

	test('searches backward', () => {
		const addon = fakeAddon();
		const search = createSearch(addon);

		search.previous('error');

		expect(addon.calls).toEqual([['prev', 'error']]);
	});

	test('does not search for an empty term', () => {
		// 空で検索すると、**端末じゅうが光ります。**押し間違いで起きる。
		const addon = fakeAddon();
		const search = createSearch(addon);

		expect(search.next('')).toBe(false);
		expect(search.next('   ')).toBe(false);
		expect(addon.calls).toEqual([]);
	});

	test('clears the highlight when the search is closed', () => {
		// **閉じたのに光ったままだと、いま何を見ているのか分からなくなります。**
		const addon = fakeAddon();
		const search = createSearch(addon);

		search.next('error');
		search.close();

		expect(addon.calls).toEqual([
			['next', 'error'],
			['clear', '']
		]);
	});

	test('survives an addon that throws instead of losing the keystroke', () => {
		// **握り潰さない。**投げたことは呼んだ側へ返す。
		const onError = vi.fn();
		const search = createSearch(
			{
				findNext: () => {
					throw new Error('描画がまだ');
				},
				findPrevious: () => false,
				clearDecorations: () => {}
			},
			onError
		);

		expect(search.next('error')).toBe(false);
		expect(onError).toHaveBeenCalled();
	});
});
