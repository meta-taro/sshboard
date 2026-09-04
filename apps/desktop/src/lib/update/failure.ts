/**
 * 更新の失敗を、人の次の一手で分ける（D34 の追記 3）。
 *
 * これまでは `String(error)` をそのまま札へ流していました。訳されないうえ、
 * **中身に URL やパスが入りえます。**この製品は画面に写り込むものを気にする
 * （CLAUDE.md 禁止事項 4・D26）のに、ここだけ素通しでした。
 *
 * **黙るのではありません。**握り潰さずに、
 * 「回線を疑う」「配布元を疑う」「分からない」の 3 つへ寄せます。
 * **人の次の一手が違うのは、この 3 つだけ**だからです。
 * 生の文字列は画面へ出さず、開発者コンソールへ落とします。
 */

export type FailureKind = 'network' | 'signature' | 'unknown';

export type FailureMessageKey =
	| 'update.failed.network'
	| 'update.failed.signature'
	| 'update.failed.unknown';

/**
 * 署名まわり。**先に見ます。**
 *
 * 「署名を取りに行けなかった」は繋がっている話ではなく配布元の話なので、
 * 回線より先に当てないと、人が回線を疑って時間を使います。
 */
const SIGNATURE = /signat|minisign|untrusted|verif|pubkey|public key/i;

/** 回線まわり。 */
const NETWORK = /network|fetch|connect|timed? ?out|timeout|dns|offline|unreachable|sending request|socket|tls|certificate/i;

/** 生の文字列を 3 つへ寄せる。 */
export function classify(raw: string): FailureKind {
	if (SIGNATURE.test(raw)) return 'signature';
	if (NETWORK.test(raw)) return 'network';
	return 'unknown';
}

/**
 * 画面へ渡す鍵。**渡すのは鍵だけ**で、生の文字列は 1 文字も持ち出しません。
 */
export function messageKeyFor(raw: string): FailureMessageKey {
	switch (classify(raw)) {
		case 'network':
			return 'update.failed.network';
		case 'signature':
			return 'update.failed.signature';
		default:
			return 'update.failed.unknown';
	}
}
