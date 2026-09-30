/**
 * **古い応答が、新しい状態を上書きしない**（Issue #8 の候補・2026-09-25 に記録）。
 *
 * `refresh()` は `session_status` を呼んで、返ってきたものをそのまま書き込みます。
 * **見張りが無いので、遅れて着いた古い応答が、後の状態を上書きできます。**
 *
 * 踏む形は 2 つ。
 *
 * 1. **移動で読めなかったとき**に `FileBrowser` が `session.refresh()` を呼びます。
 *    立て続けに動くと、**2 本目の呼びの後に 1 本目の返事が着く**ことがあります
 * 2. `session://changed` が**呼びの最中に**届くと、**届いた新しい値を、
 *    古い返事が塗り潰します**
 *
 * どちらも症状は同じ ——「**繋がっているのに、画面の接続先が実態とずれる**」。
 * **これが Issue #8 の原因だとは言えません**（実機での再現がまだです）。
 * ただし**これ自体が欠陥**なので、先に塞ぎます。
 */
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type { Opened } from './session.svelte';

const fake = vi.hoisted(() => ({
	handlers: [] as ((event: { payload: unknown }) => void)[],
	/** `session_status` が返すものを、呼ばれた順に取り出す。 */
	queue: [] as { open: unknown[]; active: string | null; after: Promise<void> | null }[]
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: async (_name: string, handler: (event: { payload: unknown }) => void) => {
		fake.handlers.push(handler);
		return () => {};
	}
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: async (command: string) => {
		if (command !== 'session_status') return undefined;
		const next = fake.queue.shift();
		if (!next) return { open: [], active: null };
		// **返すのを待たせる。**遅れて着く返事を作るため。
		if (next.after) await next.after;
		return { open: next.open, active: next.active };
	}
}));

const { session } = await import('./session.svelte');

function opened(id: string): Opened {
	return {
		id,
		name: id,
		fingerprint: 'SHA256:xxxx',
		hostKeyAlgorithm: 'ssh-ed25519',
		write: { aiRoots: [], humanUnrestricted: true }
	};
}

/** 手で開け閉めできる門。 */
function gate() {
	let open!: () => void;
	const wait = new Promise<void>((resolve) => (open = resolve));
	return { open, wait };
}

beforeEach(() => {
	fake.handlers.length = 0;
	fake.queue.length = 0;
	session.all = [];
	session.activeId = null;
});

describe('遅れて着いた応答', () => {
	test('does not overwrite the newer state when two refreshes overlap', async () => {
		// Arrange — 1 本目は足止めして、2 本目より後に返す。
		const slow = gate();
		fake.queue.push({ open: [], active: null, after: slow.wait }); // 古い（空）
		fake.queue.push({ open: [opened('web')], active: 'web', after: null }); // 新しい

		// Act
		const first = session.refresh();
		await session.refresh(); // 2 本目が先に着く
		slow.open();
		await first; // **1 本目が後から着く**

		// Assert
		expect(session.activeId).toBe('web');
		expect(session.all.map((held) => held.id)).toEqual(['web']);
	});

	test('does not overwrite what the subscription delivered while it was in flight', async () => {
		// **購読は「いま起きたこと」を運びます。**呼びの返事より新しい。
		const slow = gate();
		// 購読を張る分は、すぐ返させる（ここで足止めすると watch() が返りません）。
		fake.queue.push({ open: [], active: null, after: null });
		await session.watch();
		// ここから先の呼びだけを足止めする。
		fake.queue.push({ open: [], active: null, after: slow.wait });

		const inFlight = session.refresh();
		// 呼びの最中に、AI が繋いだことが流れてくる。
		fake.handlers[0]({ payload: { open: [opened('web')], active: 'web' } });
		slow.open();
		await inFlight;

		expect(session.activeId).toBe('web');
		expect(session.all.map((held) => held.id)).toEqual(['web']);
	});
});
