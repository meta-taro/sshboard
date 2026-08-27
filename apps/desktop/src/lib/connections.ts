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
};

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
	return '';
}

/** `.ppk` は OpenSSH 系が読めない（D19）。**登録時に気づかせる。** */
export function isPuttyKey(keyPath: string | null | undefined): boolean {
	return !!keyPath && keyPath.trim().toLowerCase().endsWith('.ppk');
}
