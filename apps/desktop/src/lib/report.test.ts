/**
 * **報告に貼る 4 行**（案・4 枚目）。
 *
 * ## ここで見るいちばん大事なこと
 *
 * **利用者名が入らないこと。**
 *
 * これは public リポジトリの Issue へ貼られる文です
 * （product-baseline §25 / `oss-privacy-check` が検出します）。
 * 案は「設定の置き場所」を含めていますが、**そのままだと
 * `/Users/<名前>/…` が入ります。**
 */
import { describe, expect, test } from 'vitest';

import { reportText, withoutHome, type ReportFacts } from './report';

const FACTS: ReportFacts = {
	version: '0.1.25',
	platform: 'macos · aarch64',
	configDir: '/Users/someone/Library/Application Support/dev.sshboard.sshboard',
	mcpRunning: true
};

describe('報告に貼る 4 行', () => {
	test('carries the version, because that is what a report needs first', () => {
		expect(reportText(FACTS)).toContain('0.1.25');
	});

	test('is exactly four lines', () => {
		// **増やすと、人は貼る前に削ります。**削られたら意味がありません。
		expect(reportText(FACTS).split('\n')).toHaveLength(4);
	});

	test('does not carry the name of the person using it', () => {
		// **ここが一番大事。**public リポへ貼られます。
		const text = reportText(FACTS);
		expect(text).not.toContain('someone');
		expect(text).toContain('~/Library/Application Support/dev.sshboard.sshboard');
	});

	test('takes the home out on every platform we ship to', () => {
		expect(withoutHome('/Users/a/x')).toBe('~/x');
		expect(withoutHome('/home/bee/x')).toBe('~/x');
		expect(withoutHome('C:\\Users\\cee\\AppData\\x')).toBe('~\\AppData\\x');
	});

	test('leaves a path it does not recognise alone', () => {
		// **壊して短くするより、そのまま出す。**
		expect(withoutHome('/opt/sshboard')).toBe('/opt/sshboard');
	});

	test('says when something is missing instead of inventing it', () => {
		// **嘘の版を出さない。**空欄を N/A で埋めない（§19）。
		const text = reportText({ version: null, platform: null, configDir: null, mcpRunning: false });
		expect(text).toContain('不明');
		expect(text).not.toContain('null');
		expect(text).not.toContain('undefined');
	});

	test('says plainly when the MCP is not listening', () => {
		expect(reportText({ ...FACTS, mcpRunning: false })).toContain('止まっています');
	});
});
