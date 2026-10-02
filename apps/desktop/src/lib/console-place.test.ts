/**
 * **端末が「いまここに無い」ことを、画面が言う**（2026-10-02・オーナーの指摘）。
 *
 * > **ヘラけるのかさえわからないよ。**
 * > しかし sshboard でやろうとしたら、**これできるのか！？から始まった**
 *
 * 端末の面には接続のタブが並びます。**押せば、その接続の端末が開くように見えます。**
 * 実際は **端末は全体で 1 本**（D29）で、タブを移しても端末は付いてきません。
 * そして**そのことが、どこにも書いてありません。**
 *
 * **機能の有無を、使う人が判断できずに止まりました。**
 * 足す／足さない以前に、**画面が答えていない**のが先の問題です。
 */
import { describe, expect, test } from 'vitest';

import { consoleElsewhere } from './console-place';

describe('端末がどこに在るか', () => {
	test('says nothing when the console belongs to the connection we are looking at', () => {
		expect(consoleElsewhere('web-prod', 'web-prod')).toBeNull();
	});

	test('names the other connection when the console is held there', () => {
		// **ここが実機で詰まった所。**db-prod のタブを見ているのに、端末は web-prod のもの。
		expect(consoleElsewhere('web-prod', 'db-prod')).toBe('web-prod');
	});

	test('says nothing when no console is open', () => {
		expect(consoleElsewhere(null, 'db-prod')).toBeNull();
	});

	test('says nothing when nothing is selected', () => {
		expect(consoleElsewhere('web-prod', null)).toBeNull();
	});
});
