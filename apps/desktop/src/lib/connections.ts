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
	return { id: '', name: '', host: '', port: 22, user: '' };
}

/** 保存してよいか。**理由を返す。**空文字なら問題なし。 */
export function whyNotSavable(entry: Connection, existingIds: readonly string[]): string {
	if (!entry.id.trim()) return '識別子を入れてください';
	if (!/^[A-Za-z0-9._-]+$/.test(entry.id)) return '識別子は英数字と . _ - だけにしてください';
	if (!entry.host.trim()) return 'ホストを入れてください';
	if (!entry.user.trim()) return 'ログイン名を入れてください';
	if (entry.port < 1 || entry.port > 65535) return 'ポートが範囲外です';
	if (existingIds.includes(entry.id)) return `識別子 ${entry.id} は既に使われています`;
	if (!isConnectionTag(entry.tag))
		return `タグは ${CONNECTION_TAG_MAX_CHARS} 文字までです`;
	return '';
}

/** `.ppk` は OpenSSH 系が読めない（D19）。**登録時に気づかせる。** */
export function isPuttyKey(keyPath: string | null | undefined): boolean {
	return !!keyPath && keyPath.trim().toLowerCase().endsWith('.ppk');
}
