import type { Page } from '@playwright/test';

/**
 * **Tauri の実行体の代わり**（画面のハーネス・2026-10-25）。
 *
 * 画面は `window.__TAURI_INTERNALS__.invoke` で実行体を呼びます。ここをブラウザの中で
 * 差し替え、**決めた返事を返し、呼ばれたものを控えます。**
 * 実機でしか出ない状態（繋いでいる・別の接続で端末が開いている など）を、
 * 窓を立てずに作って測るためです。
 *
 * 返事を決めていない命令には `null` を返します（画面は「取れなかった」として描く）。
 * **接続先は架空の識別子だけ**（禁止事項 4）。
 */
export type Replies = Record<string, unknown>;

export interface Called {
	readonly cmd: string;
	readonly args: unknown;
}

export async function fakeTauri(page: Page, replies: Replies): Promise<void> {
	await page.addInitScript((given: Replies) => {
		const calls: { cmd: string; args: unknown }[] = [];
		let nextCallback = 1;
		(window as unknown as { __fakeCalls: typeof calls }).__fakeCalls = calls;
		// 購読を外す口。**無いと、画面を離れるたびに未処理の拒否が出ます。**
		(window as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__ =
			{ unregisterListener: () => undefined };
		(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
			metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
			transformCallback: () => nextCallback++,
			unregisterCallback: () => undefined,
			convertFileSrc: (path: string) => path,
			invoke: async (cmd: string, args: unknown) => {
				calls.push({ cmd, args });
				if (cmd === 'plugin:event|listen') return nextCallback++;
				if (cmd.startsWith('plugin:')) return null;
				return cmd in given ? given[cmd] : null;
			}
		};
	}, replies);
}

/** 画面が呼んだ命令。**押した結果、実行体へ何が行ったか**を測るために。 */
export async function calledWith(page: Page, cmd: string): Promise<Called[]> {
	return page.evaluate(
		(name) =>
			(window as unknown as { __fakeCalls: Called[] }).__fakeCalls.filter((c) => c.cmd === name),
		cmd
	);
}

/** 架空の接続 1 件。`session_status` の `open` に入る形。 */
export function opened(id: string, name: string) {
	return {
		id,
		name,
		tag: null,
		color: null,
		fingerprint: 'SHA256:fake',
		hostKeyAlgorithm: 'ssh-ed25519',
		write: { aiRoots: [], humanUnrestricted: true }
	};
}
