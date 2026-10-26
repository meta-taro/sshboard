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
		// **試験の途中で返事を変えられるように**、窓に置きます（`setReply`）。
		(window as unknown as { __fakeReplies: Replies }).__fakeReplies = given;
		let nextCallback = 1;
		// **購読も本物の形で受けます。**`emitEvent` で実行体からの知らせを画面へ届けるため。
		const callbacks: Record<number, (payload: unknown) => void> = {};
		const listeners: { event: string; handler: number }[] = [];
		(window as unknown as { __fakeEmit: (event: string, payload: unknown) => void }).__fakeEmit =
			(event, payload) => {
				for (const held of listeners.filter((l) => l.event === event)) {
					callbacks[held.handler]?.({ event, id: 0, payload });
				}
			};
		(window as unknown as { __fakeListening: (event: string) => boolean }).__fakeListening = (
			event
		) => listeners.some((l) => l.event === event);
		(window as unknown as { __fakeCalls: typeof calls }).__fakeCalls = calls;
		// 購読を外す口。**無いと、画面を離れるたびに未処理の拒否が出ます。**
		(window as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__ =
			{ unregisterListener: () => undefined };
		(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
			metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
			transformCallback: (callback: (payload: unknown) => void) => {
				const id = nextCallback++;
				callbacks[id] = callback;
				return id;
			},
			unregisterCallback: () => undefined,
			convertFileSrc: (path: string) => path,
			invoke: async (cmd: string, args: unknown) => {
				calls.push({ cmd, args });
				if (cmd === 'plugin:event|listen') {
					const { event, handler } = args as { event: string; handler: number };
					listeners.push({ event, handler });
					return handler;
				}
				if (cmd.startsWith('plugin:')) return null;
				const now = (window as unknown as { __fakeReplies: Replies }).__fakeReplies;
				return cmd in now ? now[cmd] : null;
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

/** 試験の途中で返事を変える。**時間が経って実行体の状態が変わった**ことを作るため。 */
export async function setReply(page: Page, cmd: string, reply: unknown): Promise<void> {
	await page.evaluate(
		([name, value]) => {
			(window as unknown as { __fakeReplies: Replies }).__fakeReplies[name as string] = value;
		},
		[cmd, reply] as const
	);
}

/** 実行体からの知らせ（Tauri のイベント）を画面へ届ける。帯の 1 行など。 */
export async function emitEvent(page: Page, event: string, payload: unknown): Promise<void> {
	// **画面が購読を張り終えるまで待つ。**張る前に届けると、黙って落ちます。
	await page.waitForFunction(
		(name) =>
			(window as unknown as { __fakeListening: (e: string) => boolean }).__fakeListening(name),
		event
	);
	await page.evaluate(
		([name, value]) => {
			(window as unknown as { __fakeEmit: (e: string, p: unknown) => void }).__fakeEmit(
				name as string,
				value
			);
		},
		[event, payload] as const
	);
}
