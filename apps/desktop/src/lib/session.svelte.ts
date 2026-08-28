/**
 * 開いている接続の状態。**画面のどこから見ても同じ 1 つ**（PRD §4-1）。
 *
 * AI が MCP から繋いだ／切ったときも `session://changed` で流れてきます。
 * **人が知らないまま繋がっている、を作らないため**（PRD §4-2）。
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** Rust 側の `Opened` と対になる。**ホスト名も利用者名も入っていません。** */
export type Opened = {
	id: string;
	name: string;
	tag?: string | null;
	fingerprint: string;
	hostKeyAlgorithm: string;
	write: {
		/** AI が書けるディレクトリ。空 = AI は書けない。 */
		aiRoots: string[];
		/** 人が書けるか。**常に true**（PRD §3）。 */
		humanUnrestricted: boolean;
	};
};

/** リモートの 1 件。 */
export type Listed = {
	name: string;
	isDir: boolean;
	size: number;
};

class SessionState {
	open = $state<Opened | null>(null);
	/** 繋ぎに行っている最中か。**押しっぱなしで二重に繋ぎに行かないため。** */
	busy = $state(false);

	/** 変化を受け取り始める。返り値を呼ぶと購読を止める。 */
	async watch(): Promise<() => void> {
		const stop = await listen<Opened | null>('session://changed', (event) => {
			this.open = event.payload;
		});
		// 起動が速いと、購読より先に流れた分を取りこぼす。**取り直す。**
		this.open = await invoke<Opened | null>('session_status');
		return stop;
	}
}

export const session = new SessionState();

/** 親ディレクトリ。**`/` の親は `/`**（上へ行き過ぎない）。 */
export function parentOf(path: string): string {
	const trimmed = path.replace(/\/+$/, '');
	if (!trimmed) return '/';
	const cut = trimmed.lastIndexOf('/');
	if (cut <= 0) return '/';
	return trimmed.slice(0, cut);
}

/** ディレクトリと名前を繋ぐ。**`//` を作らない。** */
export function joinPath(dir: string, name: string): string {
	return `${dir.replace(/\/+$/, '')}/${name}`;
}

/** 人が読める大きさ。**1024 で割る**（ファイルの大きさなので）。 */
export function humanSize(bytes: number): string {
	const units = ['B', 'KB', 'MB', 'GB', 'TB'];
	let value = bytes;
	let unit = 0;
	while (value >= 1024 && unit < units.length - 1) {
		value /= 1024;
		unit += 1;
	}
	// B は小数を出さない。1 バイト単位で意味がある場面があるため。
	return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[unit]}`;
}

/** 手元のパスからファイル名だけを取る。**Windows の `\` も見る。** */
export function baseName(path: string): string {
	const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
	return cut < 0 ? path : path.slice(cut + 1);
}
