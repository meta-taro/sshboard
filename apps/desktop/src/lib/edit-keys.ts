/**
 * 入力欄の切り取り・コピー・貼り付け・全選択を、**OS 任せにしない**。
 *
 * 自前タイトルバー（D17）で `decorations: false` にした結果、
 * **Windows ではメニューバーごと消えます。**メニューに載せていた
 * 貼り付けの割り当ても一緒に消えるので、**実機で Ctrl+V が効きませんでした。**
 *
 * D17 の実行記録には「WebView が標準で持っているので従来どおり使える」と
 * 書きましたが、**実機で確かめずに書いた一文**でした。
 *
 * **パスフレーズが入れられない ＝ 繋げない**ので、ここは一番手前の詰まりです。
 *
 * `terminal-clipboard.ts` は **xterm 専用**です。ここは普通の入力欄の側で、
 * **端末の上では一切手を出しません**（素の `Ctrl+C` は「走っているものを止める」）。
 */

/** 何をしたいか。**該当しなければ null。** */
export type EditIntent = 'cut' | 'copy' | 'paste' | 'selectAll';

/** 判定に要るものだけ。**`KeyboardEvent` を丸ごと要求しない**（テストが重くなる）。 */
export interface EditKeyEvent {
	readonly key: string;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
	/** 入力欄の中で押されたか。**端末の上なら false。** */
	readonly inField: boolean;
}

export type EditPlatform = 'mac' | 'other';

/** キーと OS から、したいことを決める。 */
export function editIntent(event: EditKeyEvent, platform: EditPlatform): EditIntent | null {
	// **入力欄の外では何もしない。**端末は `terminal-clipboard.ts` の担当です。
	if (!event.inField) return null;

	// mac は Command、それ以外は Ctrl。**両方受けると、mac で端末の慣習と食い違います。**
	const held = platform === 'mac' ? event.metaKey : event.ctrlKey;
	if (!held) return null;

	switch (event.key.toLowerCase()) {
		case 'x':
			return 'cut';
		case 'c':
			return 'copy';
		case 'v':
			return 'paste';
		case 'a':
			return 'selectAll';
		default:
			// `+` / `-` / `0` は文字サイズ。**取り合いにしない。**
			return null;
	}
}

/** 入力欄の最小限。**`HTMLInputElement` を丸ごと要求しない。** */
export interface EditableField {
	value: string;
	selectionStart: number | null;
	selectionEnd: number | null;
	setSelectionRange(start: number, end: number): void;
	select(): void;
	dispatchEvent(event: Event): boolean;
}

/** 読み書きの口。**中身はここを通るだけで、どこにも残しません**（D14）。 */
export interface EditClipboard {
	writeText(text: string): Promise<void>;
	readText(): Promise<string>;
}

/**
 * 実際に動かす。
 *
 * **`document.execCommand` を使いません。**廃止予定であることに加え、
 * WebView によって挙動が違うためです。**自分で値を組み替えます。**
 */
export async function applyEdit(
	intent: EditIntent,
	field: EditableField,
	clipboard: EditClipboard
): Promise<void> {
	if (intent === 'selectAll') {
		field.select();
		return;
	}

	const start = field.selectionStart ?? 0;
	const end = field.selectionEnd ?? 0;
	const selected = field.value.slice(start, end);

	if (intent === 'copy') {
		if (selected) await clipboard.writeText(selected);
		return;
	}

	if (intent === 'cut') {
		if (!selected) return;
		await clipboard.writeText(selected);
		replace(field, start, end, '');
		return;
	}

	const incoming = await clipboard.readText();
	if (incoming) replace(field, start, end, incoming);
}

/**
 * 選択範囲を置き換え、**画面へ変化を伝える。**
 *
 * `input` を投げないと、Svelte の `bind:value` に届きません
 * （**打ったつもりで何も入っていない**、という一番分かりにくい壊れ方になります）。
 */
function replace(field: EditableField, start: number, end: number, text: string): void {
	field.value = field.value.slice(0, start) + text + field.value.slice(end);
	const at = start + text.length;
	field.setSelectionRange(at, at);
	field.dispatchEvent(new Event('input', { bubbles: true }));
}
