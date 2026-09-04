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
 *
 * ## D34 の追記 3（2026-09-04）— 押す材料を渡す
 *
 * 版番号だけを見せて「はい」を押させるのは、判断ではありませんでした。
 * **変更内容を出し**（`notes.ts`）、**飛ばした版を覚え**（`skipped.ts`）、
 * **失敗を人の次の一手で分け**（`failure.ts`）、
 * **落としている間も畳める**ようにしています。
 */
import { invoke } from '@tauri-apps/api/core';

import { type FailureMessageKey, messageKeyFor } from './failure';
import { readSkipped, shouldOffer, writeSkipped } from './skipped';

export type UpdateState =
	| { kind: 'idle' }
	| { kind: 'checking' }
	| { kind: 'none'; version: string }
	| { kind: 'found'; version: string; notes: string }
	| { kind: 'downloading'; version: string; percent: number | null }
	| { kind: 'ready'; version: string }
	/** **生の文字列を持ちません。**画面へ渡すのは鍵だけ（`failure.ts`）。 */
	| { kind: 'failed'; messageKey: FailureMessageKey };

let state = $state<UpdateState>({ kind: 'idle' });

/**
 * 人が「更新を確認」を押したか。
 *
 * **押されたときだけ「調べています」と「最新です」を出します。**
 * 起動時の 1 回は、何も無ければ静かなままにします — 静かなときは静かでいる。
 */
let manual = $state(false);

/**
 * 落としている間だけ札を畳んでいるか。
 *
 * **`idle` へ戻すのとは別**です。戻してしまうと、進み具合の更新で札が
 * 出たり消えたりします。畳んでいる間も裏では落ち続け、
 * **入れ終わったら（あるいは失敗したら）自分から出直します。**
 */
let folded = $state(false);

function inTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** いま名乗っている版。**取れなければ空**にして、誤った版を出しません。 */
async function currentVersion(): Promise<string> {
	try {
		return await invoke<string>('app_version');
	} catch {
		return '';
	}
}

/** 見つけた更新を握っておく。**押されてから落とす**ため。 */
let pending: { downloadAndInstall: (cb: (e: unknown) => void) => Promise<void> } | null = null;

export const updater = {
	get state(): UpdateState {
		return state;
	},

	/** 人が押して始めた確認か。**「最新です」を出してよい場面かどうか。** */
	get manual(): boolean {
		return manual;
	},

	/** 落としている間だけ畳んでいるか。 */
	get folded(): boolean {
		return folded;
	},

	/**
	 * 更新があるか調べる。**起動時に 1 回。**定期的には叩きません。
	 *
	 * `byHand` は「人が『更新を確認』を押した」の意味です。
	 * **押されたときは、飛ばした版でも出します** — 人が自分で取りに来ているので。
	 */
	async check(byHand = false): Promise<void> {
		if (!inTauri()) return;
		manual = byHand;
		folded = false;
		state = { kind: 'checking' };
		try {
			const { check } = await import('@tauri-apps/plugin-updater');
			const found = await check();
			if (!found) {
				state = { kind: 'none', version: await currentVersion() };
				return;
			}
			// **人が「この版は飛ばす」と言った版を、黙って出し直しません。**
			if (!byHand && !shouldOffer(found.version, readSkipped())) {
				state = { kind: 'none', version: await currentVersion() };
				return;
			}
			pending = found as unknown as typeof pending;
			state = { kind: 'found', version: found.version, notes: found.body ?? '' };
		} catch (error: unknown) {
			// **黙らない。**繋がらないのか、署名が合わないのかで人の次の一手が違う。
			// **生の文字列はここまで。**URL やパスが入りうるものを画面へ出しません（D26）。
			console.error('[sshboard] 更新を調べられません', error);
			state = { kind: 'failed', messageKey: messageKeyFor(String(error)) };
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
			// **畳んでいても出直します。**再起動が要ることを、伏せたままにしない。
			folded = false;
			state = { kind: 'ready', version };
		} catch (error: unknown) {
			console.error('[sshboard] 更新を入れられません', error);
			folded = false;
			state = { kind: 'failed', messageKey: messageKeyFor(String(error)) };
		}
	},

	/** 入れ終わったあと、人が押したら再起動する。**勝手に落としません。** */
	async restart(): Promise<void> {
		if (!inTauri()) return;
		try {
			const { relaunch } = await import('@tauri-apps/plugin-process');
			await relaunch();
		} catch (error: unknown) {
			console.error('[sshboard] 再起動できません', error);
			state = { kind: 'failed', messageKey: messageKeyFor(String(error)) };
		}
	},

	/**
	 * 落としている間だけ札を畳む。**中止ではありません。**
	 *
	 * 中断できるかを確かめていないので、「中止」とは名乗りません。
	 * **押しても止まらないボタンは、無いより悪い。**
	 */
	fold(): void {
		folded = true;
	},

	/** この版は出さないと覚える。**次の版が出たら、また出します。** */
	skip(): void {
		if (state.kind === 'found') writeSkipped(state.version);
		folded = false;
		manual = false;
		state = { kind: 'idle' };
	},

	dismiss(): void {
		folded = false;
		manual = false;
		state = { kind: 'idle' };
	}
};
