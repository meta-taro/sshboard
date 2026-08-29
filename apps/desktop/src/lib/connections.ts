/** 登録された接続 1 件。Rust 側の `ConnectionEntry` と対になる。 */
export type Connection = {
	id: string;
	name: string;
	host: string;
	port: number;
	user: string;
	key_path?: string | null;
	keyring_passphrase_ref?: string | null;
	fingerprint?: string | null;
	known_hosts?: string | null;
	/** 印の色。**16 進数ではなく名前**（配色側が明暗を選ぶため）。 */
	color?: string | null;
	/** 印のタグ。`本番` / `開発2` など。**12 文字まで。** */
	tag?: string | null;
	/**
	 * **AI が書いてよいディレクトリ**（D22）。空 ＝ AI は 1 バイトも書けない。
	 * 人（GUI）はこれに関係なく自由に書ける（PRD §3）。
	 */
	write_roots?: string[];
};

/**
 * 付けられる色。**並び順は選択画面に出る順**（スペクトル順なので、
 * 8 つのラベルを読まなくても「緑のやつ」を探せる）。
 * Rust 側の `CONNECTION_COLORS` と同じ並び。
 */
export const CONNECTION_COLORS = [
	'red',
	'orange',
	'amber',
	'yellow',
	'lime',
	'green',
	'emerald',
	'teal',
	'cyan',
	'sky',
	'blue',
	'indigo',
	'violet',
	'purple',
	'magenta',
	'pink',
] as const;

/** タグの上限。**バイトではなく文字数。**漢字 12 文字は 36 バイト。 */
export const CONNECTION_TAG_MAX_CHARS = 12;

/** タグが行に載る長さか。空は「タグ無し」として通す。 */
export function isConnectionTag(tag: string | null | undefined): boolean {
	return [...(tag ?? '')].length <= CONNECTION_TAG_MAX_CHARS;
}

/** 新規登録の初期値。**ssh-agent を既定にする**（鍵のパスを空にしておく・D11）。 */
export function emptyConnection(): Connection {
	// **書き込み許可は空で始める**（D22）。既定で書けるようにしない。
	return { id: '', name: '', host: '', port: 22, user: '', write_roots: [] };
}

/** 保存してよいか。**理由を返す。**空文字なら問題なし。 */
/**
 * 保存してよいか。**返すのは理由ではなく鍵。**
 *
 * 文言をここに書くと、言語を切り替えても変わらない場所ができます。
 * **訳は画面側で当てます。**
 */
export type SaveBlocker =
	| { key: 'conn.err.id.empty' }
	| { key: 'conn.err.id.chars' }
	| { key: 'conn.err.host' }
	| { key: 'conn.err.user' }
	| { key: 'conn.err.port' }
	| { key: 'conn.err.dup'; id: string }
	| { key: 'conn.err.tag'; max: number }
	| null;

export function whyNotSavable(
	entry: Connection,
	existingIds: readonly string[]
): SaveBlocker {
	if (!entry.id.trim()) return { key: 'conn.err.id.empty' };
	if (!/^[A-Za-z0-9._-]+$/.test(entry.id)) return { key: 'conn.err.id.chars' };
	if (!entry.host.trim()) return { key: 'conn.err.host' };
	if (!entry.user.trim()) return { key: 'conn.err.user' };
	if (entry.port < 1 || entry.port > 65535) return { key: 'conn.err.port' };
	if (existingIds.includes(entry.id)) return { key: 'conn.err.dup', id: entry.id };
	if (!isConnectionTag(entry.tag)) return { key: 'conn.err.tag', max: CONNECTION_TAG_MAX_CHARS };
	return null;
}

/**
 * 鍵ファイルについて Rust 側が返すこと（D28）。
 *
 * **判定は中身で行われます。**拡張子は見ません — `*.tera.ppk` の中身が
 * OpenSSH 秘密鍵だった、が実際に在りました。
 */
export type KeyReport = {
	/** ファイルとして読めたか。**無いのか、形式が違うのかを分ける。** */
	readable: boolean;
	/** 認証に使えるか。公開鍵と、鍵でないものは使えない。 */
	usable: boolean;
	needsPassphrase: boolean;
	/** 人に見せる形式の名前（`OpenSSH` / `PuTTY (PPK v3)` など）。 */
	format: string;
};

/**
 * 画面に出す 1 行。**強さまでここで決める**（画面側で分岐させない）。
 *
 * `key` を `string` にしないこと。**存在しない文言キーを渡せてしまい、
 * 型検査を通ったまま画面から文字が消えます**（メニューで 1 回踏んでいます）。
 */
export type KeyNotice =
	| { tone: 'none' }
	| {
			tone: 'info' | 'error';
			key: 'conn.key.missing' | 'conn.key.unusable' | 'conn.key.ok' | 'conn.key.passphrase';
			format: string;
	  };

/**
 * 判定の結果を、人に見せる 1 行へ落とす。
 *
 * **PuTTY 形式を特別扱いしません**（D28 が D19 を覆した点）。
 * `russh` がそのまま読めるので、変換を求める理由がありません。
 */
export function keyNotice(
	keyPath: string | null | undefined,
	report: KeyReport | null
): KeyNotice {
	if (!keyPath || !keyPath.trim() || !report) return { tone: 'none' };

	if (!report.readable) return { tone: 'error', key: 'conn.key.missing', format: '' };
	if (!report.usable) {
		return { tone: 'error', key: 'conn.key.unusable', format: report.format };
	}
	return {
		tone: 'info',
		key: report.needsPassphrase ? 'conn.key.passphrase' : 'conn.key.ok',
		format: report.format
	};
}
