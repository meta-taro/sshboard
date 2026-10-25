import { defineConfig, devices } from '@playwright/test';

/**
 * **本物のブラウザで画面の配置を測る**（2026-10-25）。
 *
 * vitest の試験はソースの文字列しか見ません。0.1.27 で
 * **Windows の窓から右上の × が消えた**のは、その試験を全部通したうえでの退行でした。
 * 配置は、描いて測るまで分かりません。
 *
 * - **chromium** … Windows の WebView2 とほぼ同じ描き方（D66）
 * - **webkit** … macOS の Tauri（WKWebView）と同じ描き方
 *
 * Tauri の外で描くので、上に「MCP の状態を取得できません」の帯が出ます。
 * 測るのは窓の枠（ヘッダー）なので、それで構いません。
 */
const PORT = 14200;

export default defineConfig({
	testDir: 'tests/layout',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	reporter: process.env.CI ? 'list' : 'line',
	use: {
		baseURL: `http://127.0.0.1:${PORT}`
	},
	projects: [
		{ name: 'chromium', use: { ...devices['Desktop Chrome'] } },
		{ name: 'webkit', use: { ...devices['Desktop Safari'] } }
	],
	webServer: {
		command: `pnpm exec vite dev --port ${PORT} --strictPort`,
		url: `http://127.0.0.1:${PORT}`,
		reuseExistingServer: !process.env.CI,
		timeout: 120_000
	}
});
