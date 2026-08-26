import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

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
	}
});
