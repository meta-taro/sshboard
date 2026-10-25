/**
 * **MCP の釦が写す 1 行**（2026-10-25・Windows 実機）。
 *
 * 「`/mcp` しても一覧に出てこない」と言われました。
 * `claude mcp add` の既定の範囲は **local ＝ そのコマンドを打ったフォルダだけ**です。
 * 別のフォルダで Claude Code を開くと、sshboard は**一覧にすら出ません**
 * （繋がらない口は ✘ で並ぶので、出ないのは「登録が無い」です）。
 *
 * この Mac で確かめた: 一時フォルダで登録 → そこでは ✔ Connected、
 * リポのフォルダでは 0 件。
 */
import { describe, expect, test } from 'vitest';

import { mcpRegisterCommand } from './register-command';

const URL = 'http://127.0.0.1:22022/mcp';
const TOKEN = 'test-token-not-a-secret';

describe('MCP の登録コマンド', () => {
	test('registers for every folder, not only the one it was typed in', () => {
		expect(mcpRegisterCommand(URL, TOKEN)).toContain('--scope user');
	});

	test('says it is HTTP, since the default is stdio', () => {
		expect(mcpRegisterCommand(URL, TOKEN)).toContain('--transport http');
	});

	test('points at the URL the app reports, path included', () => {
		expect(mcpRegisterCommand(URL, TOKEN)).toContain(` sshboard ${URL} `);
	});

	test('passes the token as a bearer header', () => {
		expect(mcpRegisterCommand(URL, TOKEN)).toContain(
			`--header "Authorization: Bearer ${TOKEN}"`
		);
	});

	test('puts the options before the name, where claude reads them', () => {
		const line = mcpRegisterCommand(URL, TOKEN);
		expect(line.indexOf('--scope user')).toBeLessThan(line.indexOf(' sshboard '));
	});
});
