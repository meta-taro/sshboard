/**
 * 端末のコピー & ペースト。**画面を開かなくても確かめられる部分。**
 *
 * ここで一番大事なのは **素の Ctrl+C を横取りしないこと**です。
 * 端末の Ctrl+C は「動いているものを止める」で、**コピーではありません。**
 * 横取りすると、暴走したプロセスを止められなくなります。
 */
import { describe, expect, test, vi } from 'vitest';

import {
	attachClipboard,
	detectPlatform,
	isCopyShortcut,
	isPasteShortcut,
	type ClipboardTerminal,
	type ShortcutEvent
} from './terminal-clipboard';

function keydown(part: Partial<ShortcutEvent>): ShortcutEvent {
	return {
		type: 'keydown',
		key: 'a',
		ctrlKey: false,
		metaKey: false,
		shiftKey: false,
		...part
	};
}

describe('isCopyShortcut', () => {
	test('never treats a bare Ctrl+C as copy', () => {
		// **これを取ると、走っているものを止められなくなります。**
		const event = keydown({ key: 'c', ctrlKey: true });

		expect(isCopyShortcut(event, 'mac')).toBe(false);
		expect(isCopyShortcut(event, 'other')).toBe(false);
	});

	test('takes Command+C on macOS', () => {
		expect(isCopyShortcut(keydown({ key: 'c', metaKey: true }), 'mac')).toBe(true);
	});

	test('takes Ctrl+Shift+C where there is no Command key', () => {
		// Windows / Linux の端末はこの組み合わせ。**Ctrl+C を空けておくため。**
		expect(isCopyShortcut(keydown({ key: 'C', ctrlKey: true, shiftKey: true }), 'other')).toBe(
			true
		);
	});

	test('ignores anything that is not a key press', () => {
		// keydown / keyup / keypress が全部同じ口を通るので、**押した瞬間だけ見る。**
		const released = { ...keydown({ key: 'c', metaKey: true }), type: 'keyup' };

		expect(isCopyShortcut(released, 'mac')).toBe(false);
	});
});

describe('isPasteShortcut', () => {
	test('takes Ctrl+Shift+V', () => {
		expect(isPasteShortcut(keydown({ key: 'V', ctrlKey: true, shiftKey: true }))).toBe(true);
	});

	test('leaves Command+V alone so it is not pasted twice', () => {
		// **メニューの「ペースト」が OS のロールで動いています**（menu.rs）。
		// ここでも拾うと、**1 回の ⌘V で 2 回貼られます。**
		expect(isPasteShortcut(keydown({ key: 'v', metaKey: true }))).toBe(false);
	});

	test('never treats a bare Ctrl+V as paste', () => {
		// 端末の Ctrl+V は「次の 1 文字をそのまま入れる」。**潰さない。**
		expect(isPasteShortcut(keydown({ key: 'v', ctrlKey: true }))).toBe(false);
	});
});

describe('detectPlatform', () => {
	test('reads macOS from the platform string', () => {
		expect(detectPlatform('MacIntel')).toBe('mac');
		expect(detectPlatform('Win32')).toBe('other');
		expect(detectPlatform('Linux x86_64')).toBe('other');
	});
});

/** 差し替えられる端末。**本物の xterm を持ち込まずに確かめる。** */
function fakeTerminal(selection = ''): ClipboardTerminal & {
	selectionChanged: () => void;
	press: (event: ShortcutEvent) => boolean;
	pasted: string[];
} {
	let notify = () => {};
	let handler: (event: ShortcutEvent) => boolean = () => true;
	const pasted: string[] = [];

	return {
		getSelection: () => selection,
		onSelectionChange: (callback: () => void) => {
			notify = callback;
			return { dispose: () => {} };
		},
		attachCustomKeyEventHandler: (next: (event: ShortcutEvent) => boolean) => {
			handler = next;
		},
		paste: (text: string) => {
			pasted.push(text);
		},
		selectionChanged: () => notify(),
		press: (event: ShortcutEvent) => handler(event),
		pasted
	};
}

describe('attachClipboard', () => {
	test('copies what the mouse selected, without any key press', async () => {
		// **なぞるだけでコピー。**端末はこれが当たり前で、無いと毎回ためらう。
		const terminal = fakeTerminal('/var/log/messages');
		const writeText = vi.fn().mockResolvedValue(undefined);
		attachClipboard(terminal, { writeText, readText: vi.fn(), onError: vi.fn() }, 'mac');

		terminal.selectionChanged();
		await Promise.resolve();

		expect(writeText).toHaveBeenCalledWith('/var/log/messages');
	});

	test('does not touch the clipboard when the selection is cleared', async () => {
		// なぞり直すたびに空で上書きすると、**さっきコピーしたものが消えます。**
		const terminal = fakeTerminal('');
		const writeText = vi.fn().mockResolvedValue(undefined);
		attachClipboard(terminal, { writeText, readText: vi.fn(), onError: vi.fn() }, 'mac');

		terminal.selectionChanged();
		await Promise.resolve();

		expect(writeText).not.toHaveBeenCalled();
	});

	test('lets a bare Ctrl+C through to the shell', () => {
		// **止められること。**ここが通らないと、端末として使えません。
		const terminal = fakeTerminal('選択されている文字列');
		attachClipboard(terminal, { writeText: vi.fn(), readText: vi.fn(), onError: vi.fn() }, 'other');

		const passedThrough = terminal.press(keydown({ key: 'c', ctrlKey: true }));

		expect(passedThrough).toBe(true);
	});

	test('swallows the copy shortcut so it does not also reach the shell', () => {
		const terminal = fakeTerminal('選択されている文字列');
		const writeText = vi.fn().mockResolvedValue(undefined);
		attachClipboard(terminal, { writeText, readText: vi.fn(), onError: vi.fn() }, 'mac');

		const passedThrough = terminal.press(keydown({ key: 'c', metaKey: true }));

		expect(passedThrough).toBe(false);
		expect(writeText).toHaveBeenCalledWith('選択されている文字列');
	});

	test('leaves the copy shortcut alone when nothing is selected', () => {
		// 何も選んでいないのに握り込むと、**その組み合わせが死にます。**
		const terminal = fakeTerminal('');
		const writeText = vi.fn();
		attachClipboard(terminal, { writeText, readText: vi.fn(), onError: vi.fn() }, 'mac');

		const passedThrough = terminal.press(keydown({ key: 'c', metaKey: true }));

		expect(passedThrough).toBe(true);
		expect(writeText).not.toHaveBeenCalled();
	});

	test('pastes what the clipboard holds', async () => {
		const terminal = fakeTerminal();
		const readText = vi.fn().mockResolvedValue('systemctl status nginx');
		attachClipboard(terminal, { writeText: vi.fn(), readText, onError: vi.fn() }, 'other');

		terminal.press(keydown({ key: 'V', ctrlKey: true, shiftKey: true }));
		await Promise.resolve();
		await Promise.resolve();

		expect(terminal.pasted).toEqual(['systemctl status nginx']);
	});

	test('does not paste into a pane that cannot be typed into', async () => {
		// 出力の面は**見るだけ**です（`disableStdin`）。そこで貼れてしまうと、
		// **打てないはずの面から文字がサーバーへ行きます。**なぞってコピーは残す。
		const terminal = fakeTerminal('見えている行');
		const readText = vi.fn().mockResolvedValue('rm -rf /');
		const writeText = vi.fn().mockResolvedValue(undefined);
		attachClipboard(terminal, { writeText, readText, onError: vi.fn() }, 'other', {
			allowPaste: false
		});

		const passedThrough = terminal.press(keydown({ key: 'V', ctrlKey: true, shiftKey: true }));
		await Promise.resolve();
		await Promise.resolve();

		expect(terminal.pasted).toEqual([]);
		expect(readText).not.toHaveBeenCalled();
		expect(passedThrough).toBe(true);

		// **コピーの側は生きている。**
		terminal.selectionChanged();
		await Promise.resolve();
		expect(writeText).toHaveBeenCalledWith('見えている行');
	});

	test('lets another owner take a key first, without knowing what it is for', () => {
		// xterm のキー処理は **1 本しか付けられません。**検索もここを通る必要がある。
		// クリップボード側は「誰かが処理した」しか知りません（検索を知らない）。
		const terminal = fakeTerminal();
		const taken: string[] = [];
		attachClipboard(terminal, { writeText: vi.fn(), readText: vi.fn(), onError: vi.fn() }, 'mac', {
			handledElsewhere: (event) => {
				if (event.key !== 'f') return false;
				taken.push(event.key);
				return true;
			}
		});

		const passedThrough = terminal.press(keydown({ key: 'f', metaKey: true }));

		expect(taken).toEqual(['f']);
		expect(passedThrough).toBe(false);
	});

	test('says so when the clipboard refuses, instead of doing nothing', async () => {
		// **握り潰さない。**押したのに何も起きないと、人は壊れたと思います。
		const terminal = fakeTerminal();
		const readText = vi.fn().mockRejectedValue(new Error('クリップボードを読めません'));
		const onError = vi.fn();
		attachClipboard(terminal, { writeText: vi.fn(), readText, onError }, 'other');

		terminal.press(keydown({ key: 'V', ctrlKey: true, shiftKey: true }));
		await Promise.resolve();
		await Promise.resolve();

		expect(onError).toHaveBeenCalled();
	});
});
