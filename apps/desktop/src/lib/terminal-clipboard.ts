/**
 * 端末のコピー & ペースト。
 *
 * **なぞるだけでコピー**（Tera Term / PuTTY と同じ）と、
 * **端末を壊さないショートカット**を足します。
 *
 * ## 触ってはいけない組み合わせ
 *
 * | 組み合わせ | 端末での意味 | ここでの扱い |
 * |---|---|---|
 * | `Ctrl+C` | **走っているものを止める**（SIGINT） | **横取りしない** |
 * | `Ctrl+V` | 次の 1 文字をそのまま入れる | **横取りしない** |
 * | `⌘V` | メニューの「ペースト」が OS のロールで処理する（`menu.rs`） | **横取りしない**（拾うと 2 回貼られる） |
 *
 * **`Ctrl+C` を「コピー」にしてしまうと、暴走したプロセスを止められません。**
 * だから Windows / Linux 側のコピーは `Ctrl+Shift+C` です。
 */

/** どちらの流儀のキーボードか。**`⌘` があるかどうかだけ。** */
export type TerminalPlatform = 'mac' | 'other';

/** 判定に要る分だけ。**本物の `KeyboardEvent` を作らずに確かめるため。** */
export interface ShortcutEvent {
	readonly type: string;
	readonly key: string;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
}

/** 差し替えられる端末。**本物の xterm もこの形を満たします。** */
export interface ClipboardTerminal {
	getSelection(): string;
	onSelectionChange(callback: () => void): { dispose(): void };
	attachCustomKeyEventHandler(handler: (event: ShortcutEvent) => boolean): void;
	paste(text: string): void;
}

/** クリップボードの口。**失敗を握り潰さないため `onError` を必ず持ちます。** */
export interface ClipboardPorts {
	readonly writeText: (text: string) => Promise<void>;
	readonly readText: () => Promise<string>;
	readonly onError: (error: unknown) => void;
}

export function detectPlatform(platform: string): TerminalPlatform {
	return /mac/i.test(platform) ? 'mac' : 'other';
}

/**
 * コピーの組み合わせか。
 *
 * **素の `Ctrl+C` は、どちらの流儀でも常に `false`。**
 */
export function isCopyShortcut(event: ShortcutEvent, platform: TerminalPlatform): boolean {
	if (event.type !== 'keydown') return false;
	if (event.key.toLowerCase() !== 'c') return false;

	if (platform === 'mac') {
		return event.metaKey && !event.ctrlKey;
	}
	// **Shift を必須にする。**`Ctrl+C` を空けておくため。
	return event.ctrlKey && event.shiftKey && !event.metaKey;
}

/**
 * ペーストの組み合わせか。**`Ctrl+Shift+V` だけ。**
 *
 * `⌘V` と `Ctrl+V` は OS のメニュー（`menu.rs` の `PredefinedMenuItem::paste`）が
 * すでに処理します。**ここでも拾うと、1 回押して 2 回貼られます。**
 */
export function isPasteShortcut(event: ShortcutEvent): boolean {
	if (event.type !== 'keydown') return false;
	if (event.key.toLowerCase() !== 'v') return false;

	return event.ctrlKey && event.shiftKey && !event.metaKey;
}

/** 面ごとの違い。**出力を見るだけの面には貼り付けを付けません。** */
export interface ClipboardOptions {
	/** 打てない面（`disableStdin`）では `false`。**既定は打てる面。** */
	readonly allowPaste?: boolean;
}

/**
 * 端末へコピー & ペーストを付ける。**外すための関数を返します。**
 */
export function attachClipboard(
	terminal: ClipboardTerminal,
	ports: ClipboardPorts,
	platform: TerminalPlatform,
	options: ClipboardOptions = {}
): () => void {
	const allowPaste = options.allowPaste ?? true;
	// **なぞり終えた時点でコピー。**キーを押させない。
	const selection = terminal.onSelectionChange(() => {
		const selected = terminal.getSelection();
		// 空で上書きすると、**さっきコピーしたものが消えます。**
		if (!selected) return;
		ports.writeText(selected).catch(ports.onError);
	});

	terminal.attachCustomKeyEventHandler((event) => {
		if (isCopyShortcut(event, platform)) {
			const selected = terminal.getSelection();
			// 何も選んでいなければ**その組み合わせを塞がない。**
			if (!selected) return true;
			ports.writeText(selected).catch(ports.onError);
			return false;
		}

		if (allowPaste && isPasteShortcut(event)) {
			ports
				.readText()
				.then((text) => {
					if (text) terminal.paste(text);
				})
				.catch(ports.onError);
			return false;
		}

		// **それ以外は全部シェルへ。**Ctrl+C もここを通ります。
		return true;
	});

	return () => selection.dispose();
}

/**
 * ブラウザのクリップボードを使う口。
 *
 * **読み取りは許可を求められることがあります**（WebView の作り次第）。
 * 断られたら `onError` に出して、**押したのに何も起きない状態にしません。**
 */
export function browserClipboard(onError: (error: unknown) => void): ClipboardPorts {
	return {
		writeText: (text: string) => navigator.clipboard.writeText(text),
		readText: () => navigator.clipboard.readText(),
		onError
	};
}
