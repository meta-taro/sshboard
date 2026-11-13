/**
 * **ファイルのタブを、この接続・このフォルダで開いて**という頼み（2026-11-13・PdM の依頼 #2035）。
 *
 * MCP の `show_view files` に `connection_id` / `path` / `select` が付いて届いたとき、
 * ページがここへ置き、`FileBrowser` が拾って開きます。
 *
 * **フォルダが在るか・読めるかは本体が先に確かめています**（`open_files.rs`）。
 * ここは画面を動かすだけで、**選ぶだけ**（開く・落とす・消すはしない）。
 *
 * `FileBrowser` は［ファイル］のタブを開いたときに作られるので、頼みはタブより先に届きます。
 * だから**置いておいて、作られたときに拾う**形にしています。
 */

export interface FilesRequest {
	connectionId: string;
	path?: string;
	select?: string;
}

/** 本体から届く形（`crates/sshboard-mcp` の `AgentAct::Open`）。 */
export interface OpenAct {
	action: 'open';
	connection_id: string;
	path?: string;
	select?: string;
}

export const filesRequest = $state<{ pending: FilesRequest | null }>({ pending: null });

/** 頼みを置く。**後から来た頼みが前の頼みを置き換えます**（最後に言われた所を開く）。 */
export function askFiles(act: OpenAct): void {
	filesRequest.pending = {
		connectionId: act.connection_id,
		path: act.path,
		select: act.select
	};
}

/** 頼みを受け取る。**受け取ったら消えます**（同じ頼みで 2 回動かない）。 */
export function takeFilesRequest(): FilesRequest | null {
	const taken = filesRequest.pending;
	filesRequest.pending = null;
	return taken;
}
