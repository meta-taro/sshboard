/**
 * 立ち上げ直したあとの繋ぎ直し（UI 案・5 枚目）。
 *
 * ## ここで見るいちばん大事なこと
 *
 * **信じられない控えで人に問わない。**
 * 「0 件へ繋ぎ直しますか」「`undefined` へ繋ぎ直しますか」を出すと、
 * 人は**押せない問い**の前で止まります。壊れていたら**出しません。**
 *
 * **そして控えには接続先を入れません**（禁止事項 4 / §14）——
 * 識別子は人が付けた名札で、それ自体は接続先を明かしません。
 */
import { afterEach, beforeEach, describe, expect, test } from 'vitest';

import { forget, remember, STILL_FRESH_MS, take } from './reopen';

/** `localStorage` の最小実装（`splitter.test.ts` と同じ作法）。 */
class MemoryStorage {
	private held = new Map<string, string>();
	getItem(key: string): string | null {
		return this.held.get(key) ?? null;
	}
	setItem(key: string, value: string): void {
		this.held.set(key, value);
	}
	removeItem(key: string): void {
		this.held.delete(key);
	}
	clear(): void {
		this.held.clear();
	}
	get length(): number {
		return this.held.size;
	}
	key(): string | null {
		return null;
	}
}

const NOW = 1_700_000_000_000;

beforeEach(() => {
	globalThis.localStorage = new MemoryStorage() as unknown as Storage;
});

afterEach(() => {
	// **差し替えた偽物に `clear` が無いことがある**（投げる `localStorage` の試験）。
	// 後片付けで落ちると、**本体ではなく試験の都合で赤くなります。**
	globalThis.localStorage = new MemoryStorage() as unknown as Storage;
});

describe('控える', () => {
	test('remembers the identifiers that were open', () => {
		remember(['web-01', 'db-01'], NOW);
		expect(take(NOW + 1000)).toEqual({ at: NOW, ids: ['web-01', 'db-01'] });
	});

	test('writes nothing when nothing was open', () => {
		// **空の控えを残さない。**
		//
		// `take` の側にも守りが在るので、**出るかどうかだけ見ても素通りします**
		// （実際に素通りしました）。**控えそのものが残っていないこと**を見ます ——
		// 残すと、読み方を直した日に「0 件へ繋ぎ直しますか」が出ます。
		remember([], NOW);
		expect(localStorage.getItem('sshboard-reopen')).toBeNull();
		expect(take(NOW)).toBeNull();
	});

	test('clears an older note when nothing is open now', () => {
		// 前の控えが残っていると、**開いていないのに前回の分を聞かれます。**
		remember(['web-01'], NOW);
		remember([], NOW);
		expect(localStorage.getItem('sshboard-reopen')).toBeNull();
	});

	test('does not keep anything but the identifier', () => {
		// **接続先を入れない**（禁止事項 4）。控えの中身をそのまま見ます。
		remember(['web-01'], NOW);
		const raw = localStorage.getItem('sshboard-reopen') ?? '';
		expect(raw).toContain('web-01');
		for (const leak of ['@', '/home', 'ssh-', 'id_ed25519']) {
			expect(raw.includes(leak), `控えに ${leak} が入っています: ${raw}`).toBe(false);
		}
	});

	test('replaces the previous note instead of piling up', () => {
		remember(['web-01'], NOW);
		remember(['db-01'], NOW);
		expect(take(NOW)?.ids).toEqual(['db-01']);
	});
});

describe('読む', () => {
	test('asks only once', () => {
		// **読んだら消す。**残すと、起動のたびに同じことを聞かれます。
		remember(['web-01'], NOW);
		expect(take(NOW)).not.toBeNull();
		expect(take(NOW)).toBeNull();
	});

	test('forgets a note that is no longer about this restart', () => {
		// 翌朝に「昨日の夜に開いていたものへ繋ぎ直しますか」は、
		// **人には何の話か分かりません。**
		remember(['web-01'], NOW);
		expect(take(NOW + STILL_FRESH_MS + 1)).toBeNull();
	});

	test('keeps one that is still about this restart', () => {
		remember(['web-01'], NOW);
		expect(take(NOW + STILL_FRESH_MS - 1)).not.toBeNull();
	});

	test('throws away a note from the future', () => {
		// **時計が戻った機械**で、控えが永久に残るのを防ぎます。
		remember(['web-01'], NOW + 60_000);
		expect(take(NOW)).toBeNull();
	});

	test('does not ask about a broken note', () => {
		for (const bad of [
			'ずれた文字列',
			'{}',
			'[]',
			'null',
			'{"at":"いつか","ids":["web-01"]}',
			'{"at":1700000000000}',
			'{"at":1700000000000,"ids":"web-01"}',
			'{"at":1700000000000,"ids":[]}',
			'{"at":1700000000000,"ids":[null,123]}',
			'{"at":1700000000000,"ids":["  "]}'
		]) {
			localStorage.setItem('sshboard-reopen', bad);
			expect(take(NOW), `壊れた控えで問うている: ${bad}`).toBeNull();
		}
	});

	test('keeps the good identifiers out of a mixed note', () => {
		// 一部が壊れていても、**読めた分で聞けます** ——
		// 1 件壊れただけで全部捨てると、人は何も聞かれません。
		localStorage.setItem(
			'sshboard-reopen',
			JSON.stringify({ at: NOW, ids: ['web-01', null, '', 'db-01'] })
		);
		expect(take(NOW)?.ids).toEqual(['web-01', 'db-01']);
	});
});

describe('捨てる', () => {
	test('leaves nothing behind when the person says no', () => {
		remember(['web-01'], NOW);
		forget();
		expect(take(NOW)).toBeNull();
	});
});

describe('書けない機械でも落ちない', () => {
	test('survives a storage that throws', () => {
		// 読み書きが禁じられている場でも、**立ち上げ直し自体は止めません。**
		globalThis.localStorage = {
			getItem() {
				throw new Error('だめ');
			},
			setItem() {
				throw new Error('だめ');
			},
			removeItem() {
				throw new Error('だめ');
			}
		} as unknown as Storage;

		expect(() => remember(['web-01'], NOW)).not.toThrow();
		expect(take(NOW)).toBeNull();
		expect(() => forget()).not.toThrow();
	});
});
