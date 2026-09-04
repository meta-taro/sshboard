/**
 * 共有している接続状態の**購読**（Issue #8）。
 *
 * 画面が実態とずれないための土台です。ここが止まると、
 * **AI が繋いだ／切ったことを人が知らないまま**になります（PRD §4-2）。
 *
 * 見ているのは 3 つ。
 * 1. 二重に張らない（重なると同じ値を何度も書き、止め忘れが積み上がる）
 * 2. **張り終わる前に止めても、購読が残らない**（`onMount` の後始末は同期で走る）
 * 3. 宛先が閉じられたら、残っている 1 本へ移す（宛先が空のまま開いている、を作らない）
 */
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type { Opened } from './session.svelte';

type Handler = (event: { payload: Opened[] }) => void;

/**
 * `vi.mock` は巻き上げられるので、**中から外の変数を触れません。**
 * `vi.hoisted` で先に作ります。
 */
const fake = vi.hoisted(() => ({
	/** 張られた購読の受け口。**張った数がそのまま出る。** */
	handlers: [] as ((event: { payload: unknown }) => void)[],
	/** 止められた回数。 */
	stopped: 0,
	/** `listen` を待たせるための足止め。null なら即座に返る。 */
	gate: null as null | { open: () => void; wait: Promise<void> },
	/** `session_status` が返すもの。 */
	status: { open: [] as unknown[], active: null as string | null }
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: async (_name: string, handler: (event: { payload: unknown }) => void) => {
		if (fake.gate) await fake.gate.wait;
		fake.handlers.push(handler);
		return () => {
			fake.stopped += 1;
		};
	}
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: async (command: string) => (command === 'session_status' ? fake.status : undefined)
}));

const { session } = await import('./session.svelte');

/** 開いている 1 本。**ホスト名も利用者名も入れない**（PRD §8）。 */
function opened(id: string): Opened {
	return {
		id,
		name: id,
		fingerprint: 'SHA256:xxxx',
		hostKeyAlgorithm: 'ssh-ed25519',
		write: { aiRoots: [], humanUnrestricted: true }
	};
}

/** 足止めを作る。`open()` を呼ぶまで `listen` は返らない。 */
function gate() {
	let open = () => {};
	const wait = new Promise<void>((resolve) => {
		open = resolve;
	});
	return { open, wait };
}

/** 購読へ 1 回流す。**Rust 側の `session://changed` と同じ形。** */
function emit(payload: Opened[]): void {
	fake.handlers[0]({ payload });
}

beforeEach(() => {
	session.unwatch();
	fake.handlers = [];
	fake.stopped = 0;
	fake.gate = null;
	fake.status = { open: [], active: null };
	session.all = [];
	session.activeId = null;
});

describe('session.watch', () => {
	test('subscribes once even when called twice', async () => {
		// **画面のどこから見ても同じ 1 つ**（PRD §4-1）。
		// 呼ぶ側が増えても購読は増えないこと。増えると止め忘れが積み上がります。
		await session.watch();
		await session.watch();

		expect(fake.handlers).toHaveLength(1);
	});

	test('takes back the subscription when stopped before it finished starting', async () => {
		// `onMount` の後始末は**同期で**走るのに、`listen` は非同期です。
		// 張り終わる前に止められた分を取りこぼすと、**購読が永久に残ります。**
		fake.gate = gate();
		const starting = session.watch();

		session.unwatch();
		fake.gate.open();
		await starting;

		expect(fake.stopped).toBe(1);
	});

	test('moves the target to a surviving connection when the current one closes', async () => {
		fake.status = { open: [opened('one'), opened('two')], active: 'two' };
		await session.watch();
		expect(session.activeId).toBe('two');

		// **宛先が閉じられたら、残っている 1 本へ移す。**
		// 開いているのに何も向いていない、を作らない。
		emit([opened('one')]);

		expect(session.all.map((held) => held.id)).toEqual(['one']);
		expect(session.activeId).toBe('one');
	});
});
