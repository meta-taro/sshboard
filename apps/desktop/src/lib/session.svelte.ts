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
	/** 開いているもの**全部**（D25）。**タブに 1 本残らず出す。** */
	all = $state<Opened[]>([]);
	/** 操作の宛先。 */
	activeId = $state<string | null>(null);
	/** 繋ぎに行っている最中か。**押しっぱなしで二重に繋ぎに行かないため。** */
	busy = $state(false);

	/** いま操作の宛先になっているもの。 */
	get open(): Opened | null {
		return this.all.find((held) => held.id === this.activeId) ?? null;
	}

	/** 変化を受け取り始める。返り値を呼ぶと購読を止める。 */
	async watch(): Promise<() => void> {
		const stop = await listen<Opened[]>('session://changed', (event) => {
			this.all = event.payload;
			// 宛先が閉じられていたら、残っている 1 本へ移す。
			if (!this.all.some((held) => held.id === this.activeId)) {
				this.activeId = this.all[0]?.id ?? null;
			}
		});
		await this.refresh();
		return stop;
	}

	/** 取り直す。**起動が速いと、購読より先に流れた分を取りこぼす。** */
	async refresh(): Promise<void> {
		const status = await invoke<{ open: Opened[]; active: string | null }>('session_status');
		this.all = status.open;
		this.activeId = status.active;
	}

	/** 宛先を変える。**タブを押したとき。** */
	async focus(id: string): Promise<void> {
		await invoke('session_focus', { id });
		this.activeId = id;
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

/**
 * 手元のディレクトリと名前を繋ぐ。**Windows も相手にする**（PRD §7）。
 *
 * リモートは常に `/` ですが、手元は `\` で来ます。混ぜると
 * **落とし先が人の見ている場所と違って見える**ので、どちらか一方に決めます。
 */
export function localJoin(dir: string, name: string): string {
	const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/';
	return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
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
