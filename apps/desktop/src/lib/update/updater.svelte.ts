/**
 * 自動更新（D34）。**見つけても、黙って入れ替えません。**
 *
 * この道具は SSH の鍵を扱います。**無断で自分を書き換える**のは性格に合いません。
 * 見つけたら画面に出し、**押すのは人**。押されたら落として入れ、再起動します。
 *
 * **署名は Tauri 側が必ず検証します**（minisign・`pubkey` は `tauri.conf.json`）。
 * 検証できない更新はそもそも受け取りません。**未署名のコード署名（D12）とは別の話**で、
 * 「配布元が本物か」はこちらで担保されます。
 *
 * **Tauri の外（素の `vite dev`）では何もしません。**画面の開発に窓を要求しないため。
 */

export type UpdateState =
	| { kind: 'idle' }
	| { kind: 'checking' }
	| { kind: 'none' }
	| { kind: 'found'; version: string; notes: string }
	| { kind: 'downloading'; version: string; percent: number | null }
	| { kind: 'ready'; version: string }
	| { kind: 'failed'; detail: string };

let state = $state<UpdateState>({ kind: 'idle' });

function inTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** 見つけた更新を握っておく。**押されてから落とす**ため。 */
let pending: { downloadAndInstall: (cb: (e: unknown) => void) => Promise<void> } | null = null;

export const updater = {
	get state(): UpdateState {
		return state;
	},

	/** 更新があるか調べる。**起動時に 1 回。**定期的には叩きません。 */
	async check(): Promise<void> {
		if (!inTauri()) return;
		state = { kind: 'checking' };
		try {
			const { check } = await import('@tauri-apps/plugin-updater');
			const found = await check();
			if (!found) {
				state = { kind: 'none' };
				return;
			}
			pending = found as unknown as typeof pending;
			state = { kind: 'found', version: found.version, notes: found.body ?? '' };
		} catch (error: unknown) {
			// **黙らない。**繋がらないのか、署名が合わないのかで人の次の一手が違う。
			state = { kind: 'failed', detail: String(error) };
		}
	},

	/** 人が押したときだけ落として入れる。 */
	async install(): Promise<void> {
		if (!inTauri() || pending === null) return;
		const version = state.kind === 'found' ? state.version : '';
		state = { kind: 'downloading', version, percent: null };
		try {
			let total = 0;
			let got = 0;
			await pending.downloadAndInstall((event: unknown) => {
				const e = event as { event?: string; data?: { contentLength?: number; chunkLength?: number } };
				if (e.event === 'Started') {
					total = e.data?.contentLength ?? 0;
				} else if (e.event === 'Progress') {
					got += e.data?.chunkLength ?? 0;
					state = {
						kind: 'downloading',
						version,
						percent: total > 0 ? Math.min(100, Math.round((got / total) * 100)) : null
					};
				}
			});
			state = { kind: 'ready', version };
		} catch (error: unknown) {
			state = { kind: 'failed', detail: String(error) };
		}
	},

	/** 入れ終わったあと、人が押したら再起動する。**勝手に落としません。** */
	async restart(): Promise<void> {
		if (!inTauri()) return;
		try {
			const { relaunch } = await import('@tauri-apps/plugin-process');
			await relaunch();
		} catch (error: unknown) {
			state = { kind: 'failed', detail: String(error) };
		}
	},

	dismiss(): void {
		state = { kind: 'idle' };
	}
};
