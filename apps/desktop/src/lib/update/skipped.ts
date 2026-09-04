/**
 * 「あとで」が版を覚える（D34 の追記 3）。
 *
 * これまでの `dismiss()` は状態を戻すだけで、版を覚えていませんでした。
 * **α は毎日上げる前提（D30）なので、同じ札が毎日出ます。**
 * 出し続けられた警告は、そのうち中身を読まずに閉じられます。
 *
 * 覚えるのは**その版だけ**です。次の版が出たら、また出します。
 * 「更新そのものを止める」設定にはしません — 鍵を扱う道具を古いまま置かせない。
 *
 * 鍵の付け方は `theme.svelte.ts` / `text-size.svelte.ts` と同じ流儀です。
 */

/** 保存の鍵。**他の設定と同じ前置き。** */
export const SKIPPED_STORAGE_KEY = 'sshboard-update-skipped';

/**
 * この版を人へ出すか。
 *
 * **迷ったら出す側に倒します。**黙って更新を隠すほうが、
 * 一度多く出るより悪い（鍵を扱う道具を、古いまま気づかせない形になる）。
 */
export function shouldOffer(version: string, skipped: string | null): boolean {
	const asked = version.trim();
	const remembered = (skipped ?? '').trim();

	if (asked === '' || remembered === '') return true;
	return asked !== remembered;
}

/** 覚えているものを読む。**読めない環境では「覚えていない」。** */
export function readSkipped(): string | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		return localStorage.getItem(SKIPPED_STORAGE_KEY);
	} catch {
		// 保存領域が使えないことはあります（設定で切っている等）。
		// **そのときは毎回出す**のが、黙って隠すより安全な側。
		return null;
	}
}

/** この版を飛ばすと覚える。 */
export function writeSkipped(version: string): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(SKIPPED_STORAGE_KEY, version.trim());
	} catch {
		// 覚えられなくても更新の邪魔はしない。次の起動でまた出るだけ。
	}
}
