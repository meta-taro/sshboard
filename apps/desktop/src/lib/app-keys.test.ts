/**
 * 画面ぜんたいのキー操作が、**既に在る担当とぶつかっていないこと。**
 *
 * この製品にはキー操作の担当が 3 つ在ります ——
 * `edit-keys`（入力欄の切貼り）、`terminal-clipboard`（`Ctrl Shift C` / `V`）、
 * `terminal-search`（`Ctrl Shift F`）。
 *
 * **取り合いは押すまで分かりません。**型検査も試験も通り、
 * 「端末でコピーしたら画面が伏せられた」で初めて気づきます。
 * ここで数えておきます。
 */
import { describe, expect, test } from 'vitest';

import {
	appIntent,
	APP_SHORTCUTS,
	OTHER_SHORTCUTS,
	spellShortcut,
	type AppIntent,
	type AppKeyEvent
} from './app-keys';

// **持ち主のソースをそのまま読みます。**表が古いままになるのを防ぐため。
import clipboardSource from './terminal-clipboard?raw';
import searchSource from './terminal-search?raw';
import fileBrowserSource from './components/FileBrowser.svelte?raw';

/** 押されたキーを 1 つ作る。**既定は「飾りを何も押していない」。** */
function press(key: string, held: Partial<AppKeyEvent> = {}): AppKeyEvent {
	return {
		key,
		ctrlKey: false,
		metaKey: false,
		shiftKey: false,
		altKey: false,
		inField: false,
		...held
	};
}

describe('案が並べた 6 つ（Windows / Linux）', () => {
	test('opens the menu on Ctrl ,', () => {
		expect(appIntent(press(',', { ctrlKey: true }), 'other')).toBe('menu');
	});

	test('opens help on F1, with no modifier', () => {
		expect(appIntent(press('F1'), 'other')).toBe('help');
	});

	test('hides what is on screen on Ctrl Shift H', () => {
		expect(appIntent(press('h', { ctrlKey: true, shiftKey: true }), 'other')).toBe('mask');
	});

	test('stops the AI on Ctrl Shift .', () => {
		expect(appIntent(press('.', { ctrlKey: true, shiftKey: true }), 'other')).toBe('stopAi');
	});

	test('also takes the shifted form of that key', () => {
		// **`Shift` を押しているので、配列によっては `>` が届きます。**
		// 片方だけ受けると「効く日と効かない日がある」になります。
		expect(appIntent(press('>', { ctrlKey: true, shiftKey: true }), 'other')).toBe('stopAi');
	});

	test('adds a connection on Ctrl N', () => {
		expect(appIntent(press('n', { ctrlKey: true }), 'other')).toBe('newConnection');
	});

	test('switches screens on Ctrl 1 through 5', () => {
		// **案は 3 つ（端末 / ファイル / 接続）ですが、実物は 5 つです**
		// （＋帯・診断）。**案に無いものを消すのは再現ではない**ので 5 つまで。
		expect(appIntent(press('1', { ctrlKey: true }), 'other')).toBe('view:1');
		expect(appIntent(press('3', { ctrlKey: true }), 'other')).toBe('view:3');
		expect(appIntent(press('5', { ctrlKey: true }), 'other')).toBe('view:5');
	});

	test('does not invent a sixth screen', () => {
		expect(appIntent(press('6', { ctrlKey: true }), 'other')).toBeNull();
	});
});

describe('mac では ⌘', () => {
	test('uses Command, not Control', () => {
		expect(appIntent(press(',', { metaKey: true }), 'mac')).toBe('menu');
		expect(appIntent(press(',', { ctrlKey: true }), 'mac')).toBeNull();
	});

	test('does not take Command on the others', () => {
		expect(appIntent(press(',', { metaKey: true }), 'other')).toBeNull();
	});

	test('still wants Shift for the two that have it', () => {
		expect(appIntent(press('h', { metaKey: true, shiftKey: true }), 'mac')).toBe('mask');
		expect(appIntent(press('h', { metaKey: true }), 'mac')).toBeNull();
	});

	test('takes F1 on both, since it carries no modifier', () => {
		expect(appIntent(press('F1'), 'mac')).toBe('help');
	});
});

describe('ぶつかっていないこと', () => {
	/**
	 * **端末の担当と取り合わない。**
	 *
	 * `terminal-clipboard.ts` は `Ctrl Shift C` / `Ctrl Shift V`、
	 * `terminal-search.ts` は `Ctrl Shift F`。
	 * ここが同じ組を返すと、**端末でコピーした瞬間に画面が伏せられます。**
	 */
	test('leaves the terminal copy, paste and search combinations alone', () => {
		for (const key of ['c', 'v', 'f']) {
			expect(
				appIntent(press(key, { ctrlKey: true, shiftKey: true }), 'other'),
				`Ctrl Shift ${key.toUpperCase()} は端末の担当`
			).toBeNull();
			expect(
				appIntent(press(key, { metaKey: true, shiftKey: true }), 'mac'),
				`⌘ Shift ${key.toUpperCase()} は端末の担当`
			).toBeNull();
		}
	});

	/**
	 * **入力欄の中では何もしない。**
	 *
	 * `Ctrl N` や `Ctrl ,` を入力欄で打つ人は少ないですが、**`F1` は押されます**し、
	 * 入力欄で画面が切り替わると**打っていたものが消えます。**
	 */
	test('does nothing while the person is typing in a field', () => {
		expect(appIntent(press('F1', { inField: true }), 'other')).toBeNull();
		expect(appIntent(press('n', { ctrlKey: true, inField: true }), 'other')).toBeNull();
		expect(appIntent(press('1', { ctrlKey: true, inField: true }), 'other')).toBeNull();
		expect(appIntent(press('h', { ctrlKey: true, shiftKey: true, inField: true }), 'other')).toBeNull();
	});

	test('leaves the text size keys to the view menu', () => {
		// `+` / `-` / `0` は文字サイズ。**取り合いにしない**
		// （`edit-keys.ts` も同じ理由で避けています）。
		for (const key of ['+', '-', '0', '=']) {
			expect(appIntent(press(key, { ctrlKey: true }), 'other')).toBeNull();
		}
	});

	test('leaves F5 to the file panel', () => {
		// `FileBrowser.svelte` が「読み直す」に使っています。
		expect(appIntent(press('F5'), 'other')).toBeNull();
	});

	test('wants the modifier, so plain letters still reach the terminal', () => {
		// **素の `h` や `n` は、端末へ打たれる文字です。**
		expect(appIntent(press('h'), 'other')).toBeNull();
		expect(appIntent(press('n'), 'other')).toBeNull();
		expect(appIntent(press('.'), 'other')).toBeNull();
		expect(appIntent(press('1'), 'other')).toBeNull();
	});

	test('does not fire when Alt is also held', () => {
		// `Alt + ←` / `Alt + ↑` はファイルの面の移動です。
		expect(appIntent(press('n', { ctrlKey: true, altKey: true }), 'other')).toBeNull();
		expect(appIntent(press('h', { ctrlKey: true, shiftKey: true, altKey: true }), 'other')).toBeNull();
	});
});

describe('ヘルプの表が、実物とずれていないこと', () => {
	/**
	 * **これが一番たちの悪い壊れ方です** ——
	 * 書いてあるのに効かない／効くのに書いていない。
	 *
	 * 表からキーを組み立てて `appIntent` に通し、
	 * **本当にその動作になるか**を確かめます。
	 */
	test('every row really does what it says, on both platforms', () => {
		for (const row of APP_SHORTCUTS) {
			for (const platform of ['mac', 'other'] as const) {
				const got = appIntent(
					press(row.key, {
						ctrlKey: row.held === true && platform === 'other',
						metaKey: row.held === true && platform === 'mac',
						shiftKey: row.shift === true
					}),
					platform
				);
				expect(got, `${spellShortcut(row, platform)} が ${row.intent} にならない`).toBe(
					row.intent
				);
			}
		}
	});

	test('lists every action the module can decide', () => {
		// **動作を足して表に書き忘れる**のを塞ぎます。
		const intents: AppIntent[] = [
			'menu',
			'help',
			'mask',
			'stopAi',
			'newConnection',
			'view:1',
			'view:2',
			'view:3',
			'view:4',
			'view:5'
		];
		const listed = APP_SHORTCUTS.map((row) => row.intent);
		for (const intent of intents) {
			expect(listed, `${intent} が表に無い`).toContain(intent);
		}
		expect(listed.length).toBe(intents.length);
	});

	test('does not list the same key twice', () => {
		// 同じ組を 2 行に書くと、**どちらが効くのか表から読めません。**
		const seen = new Set<string>();
		for (const row of [...APP_SHORTCUTS, ...OTHER_SHORTCUTS]) {
			const spelled = spellShortcut(row, 'other');
			expect(seen.has(spelled), `${spelled} が 2 回出てくる`).toBe(false);
			seen.add(spelled);
		}
	});

	test('still finds each foreign key in the module that owns it', () => {
		// **持ち主が変えたのに表が古いまま**を塞ぎます。
		const sources: Record<string, string> = {
			'terminal-clipboard': clipboardSource,
			'terminal-search': searchSource,
			FileBrowser: fileBrowserSource
		};
		for (const row of OTHER_SHORTCUTS) {
			const source = sources[row.module];
			expect(source, `${row.module} のソースが読めません`).toBeTruthy();
			// `'c'` / `'F5'` / `'ArrowUp'` のような形で、必ず出てくるはず。
			expect(source.includes(`'${row.key}'`), `${row.module} に ${row.key} が無い`).toBe(true);
		}
	});
});

describe('キーの読み方', () => {
	test('says Command on a mac and Ctrl elsewhere', () => {
		const mask = APP_SHORTCUTS.find((row) => row.intent === 'mask');
		expect(mask).toBeTruthy();
		expect(spellShortcut(mask!, 'mac')).toBe('⌘ Shift H');
		expect(spellShortcut(mask!, 'other')).toBe('Ctrl Shift H');
	});

	test('leaves a key that needs no modifier alone', () => {
		const help = APP_SHORTCUTS.find((row) => row.intent === 'help');
		expect(spellShortcut(help!, 'other')).toBe('F1');
	});

	test('spells Alt for the keys the file panel owns', () => {
		const up = OTHER_SHORTCUTS.find((row) => row.key === 'ArrowUp');
		expect(spellShortcut(up!, 'other')).toBe('Alt ArrowUp');
		expect(spellShortcut(up!, 'mac')).toBe('⌥ ArrowUp');
	});
});
