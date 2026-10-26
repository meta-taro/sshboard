/**
 * **診断を接続ごとに分ける**（オーナー・2026-10-26）。
 *
 * > 診断があるなら、そもそもタブ化（接続ごとに）
 *
 * dk006 / dk008 / dk009 の行が時系列で 1 本に混ざり、**サーバごっちゃで読めない**。
 * 接続ごとのタブに分け、全体を見る「すべて」も残します。
 */
import { describe, expect, test } from 'vitest';

import { ALL, diagConnections, diagFor, type DiagEvent } from './diagnostics';

function event(seq: number, connection?: string): DiagEvent {
	return { seq, atMs: seq, level: 'info', stage: 'exec', connection, message: `m${seq}` };
}

const MIXED: DiagEvent[] = [
	event(5, 'db-01'),
	event(4, 'web-01'),
	event(3),
	event(2, 'db-01'),
	event(1, 'batch-01')
];

describe('診断を接続ごとに分ける', () => {
	test('names every connection that has a line, once each, in a stable order', () => {
		// **並びが毎回変わると、タブを押し間違えます。**名前の順に固定します。
		expect(diagConnections(MIXED)).toEqual(['batch-01', 'db-01', 'web-01']);
	});

	test('shows only that connection in its tab', () => {
		expect(diagFor(MIXED, 'db-01').map((e) => e.seq)).toEqual([5, 2]);
	});

	test('keeps everything, in the original order, under "all"', () => {
		expect(diagFor(MIXED, ALL).map((e) => e.seq)).toEqual([5, 4, 3, 2, 1]);
	});

	test('does not put a line without a connection into some connection’s tab', () => {
		// 接続の無い行（提案を押し出した など）は、**どの接続の話でもない**。
		for (const id of diagConnections(MIXED)) {
			expect(diagFor(MIXED, id).some((e) => e.connection === undefined)).toBe(false);
		}
	});

	test('has nothing to split when no line names a connection', () => {
		expect(diagConnections([event(1), event(2)])).toEqual([]);
	});
});
