import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
// **`vitest/config` の方**。`vite` の `defineConfig` は `test` を知りません。
import { defineConfig } from 'vitest/config';

// Tauri は固定ポートの dev サーバーを見に行く。ポートが勝手にずれると
// 「白い窓が出るだけ」になり、原因が掴みにくい。strictPort で落とす。
const DEV_PORT = 1420;

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// デスクトップアプリなのでサーバーは無い。静的に吐いて Tauri に読ませる。
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		port: DEV_PORT,
		strictPort: true,
		// 外から見える口を開けない（PRD §21）。
		host: '127.0.0.1'
	},
	test: {
		/*
		 * **テストを 2 つに分ける**（Issue #10 の落とし前）。
		 *
		 * 画面の部品を描くには DOM と、Svelte の**ブラウザ側**の解決が要ります。
		 * 分けずに `browser` 条件を全体へ効かせると、純粋な関数のテストまで
		 * その解決で走ることになるので、**要る所だけに効かせます。**
		 *
		 * 部品を描いて確かめるテストが 1 本も無かったせいで、
		 * **端末に 1 バイトも出ない状態が 3 日間配られました**（Issue #10）。
		 * `svelte-check` は 283 files, 0 errors のままでした。
		 * **型検査は 1 件も止めません。**
		 */
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'client',
					environment: 'jsdom',
					include: ['src/**/*.svelte.test.ts']
				},
				// **サーバー版の Svelte に解決させない。**ここは `test` の外です
				// （中へ置くと効かず、`mount(...) is not available on the server`
				// で全部落ちます — 実際に落ちました）。
				resolve: { conditions: ['browser'] }
			},
			{
				extends: './vite.config.ts',
				test: {
					name: 'node',
					environment: 'node',
					include: ['src/**/*.test.ts'],
					exclude: ['src/**/*.svelte.test.ts']
				}
			}
		]
	}
});
