/**
 * **繋ぎ損ねた理由を、人が読める字にする**（実運用の指摘・2026-09-28・Issue #26）。
 *
 * 実機の報告 ——
 *
 * > 上部に全幅で出る赤系の帯に **`[object Object]`** だけが出ています。
 * > 前後に文字はありません。ボタンもありません。
 * > **接続タブを開いた状態で、どの接続を選んでも帯は出たままです。**
 *
 * ## なぜ `[object Object]` になったか
 *
 * 実行体は、**構造を持ったまま**失敗を返します ——
 * 「ホスト鍵が初めて」「パスフレーズが要る」を、画面が**問いとして出せる**ように。
 * ところが受け取る側が `String(error)` で潰すと、**`[object Object]`** になります。
 * **JavaScript が object を文字にすると、必ずこの字**です。
 *
 * ## どこで起きたか
 *
 * ```
 * 1  人が［確かめた — 登録して繋ぐ］を押す
 * 2  host_key_trust が成功 → **指紋が書かれる**
 * 3  そのまま session_connect（passphrase: null）
 * 4  鍵にパスフレーズが要る → **構造を持ったエラー**
 * 5  catch で String(error) → **[object Object]**
 * 6  **パスフレーズの箱は開かない**（この経路では立てていなかった）
 * 7  帯は誰も消さない → **接続を選び直しても残る**
 * ```
 *
 * **「承認したのに、意味の分からない字が出たまま、先へ進めない」**という形でした。
 */

/** 実行体が返す、構造を持った失敗。**画面はこれを問いとして出せます。** */
export type ConnectFailure =
	| { kind: 'untrusted'; algorithm: string; fingerprint: string; expected: string | null }
	| { kind: 'passphraseNeeded' }
	| { kind: 'other'; message: string };

/** 構造を持っているか。**持っていれば、問いとして出せます。** */
export function asConnectFailure(error: unknown): ConnectFailure | null {
	if (typeof error !== 'object' || error === null) return null;
	const kind = (error as { kind?: unknown }).kind;
	if (kind === 'untrusted' || kind === 'passphraseNeeded' || kind === 'other') {
		return error as ConnectFailure;
	}
	return null;
}

/**
 * **人が読める字にする。**
 *
 * **`[object Object]` を出しません。**——出しても、人には何も伝わらず、
 * **こちらにも何が起きたか分かりません**（実際、この字だけで 2 日使いました）。
 */
export function readableFailure(error: unknown): string {
	const structured = asConnectFailure(error);
	if (structured) {
		if (structured.kind === 'other') return structured.message;
		// **問いとして出すべきものが、帯へ落ちてきた場合。**
		// **「何が起きたか分からない」よりは、種類だけでも出します。**
		return structured.kind;
	}
	if (error instanceof Error) return error.message;
	if (typeof error === 'string') return error;
	// **最後の砦。**ここへ来たものは `[object Object]` になりがちなので、
	// **JSON にして中身を見せます。**接続先は実行体が入れていません（PRD §8）。
	try {
		const shown = JSON.stringify(error);
		if (shown && shown !== '{}') return shown;
	} catch {
		/* 環状参照などで JSON にできない。下へ落ちます。 */
	}
	return String(error);
}
