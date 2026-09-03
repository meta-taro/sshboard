/**
 * 書き出すファイルの既定の名前（D18）。
 *
 * **日時を入れます。**入れないと `sshboard.sshbx` が毎回同じ名前になり、
 * 2 回目以降は上書きするか、手で番号を付けることになります。
 * **渡した相手の手元でも、どれがいつの分か分からなくなります。**
 *
 * 秒まで入れるのは、**同じ分に 2 回書き出すことが普通にある**ためです
 * （選び直して出し直す）。
 */

/** 例: `sshboard-20260903-1530.sshbx` */
export function defaultBundleName(now: Date = new Date()): string {
	const pad = (value: number) => String(value).padStart(2, '0');
	// **現地時刻で書きます。**受け取る人も渡す人も、自分の時計で読むためです。
	const stamp =
		`${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}` +
		`-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
	return `sshboard-${stamp}.sshbx`;
}
