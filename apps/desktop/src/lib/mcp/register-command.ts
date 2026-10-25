/**
 * MCP の釦が写す 1 行。エージェントへそのまま貼れる形にする（D23）。
 *
 * **`--scope user` を必ず付けます。**既定の local は「打ったフォルダだけ」で、
 * 別のフォルダで Claude Code を開くと `/mcp` の一覧にすら出ません（2026-10-25・Windows 実機）。
 */
export function mcpRegisterCommand(url: string, token: string): string {
	return `claude mcp add --scope user --transport http sshboard ${url} --header "Authorization: Bearer ${token}"`;
}
