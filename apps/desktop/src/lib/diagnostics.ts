/**
 * 診断の 1 件。Rust 側の `sshboard_diag::Event` と対。
 *
 * **なぜここに出したか** —— `+page.svelte` の中にインラインで書いてあり、
 * **診断の面を部品に出すと、型が 2 か所になる**所でした。
 * 2 か所に書いた型は、**片方だけ直る日**が来ます（D39）——
 * しかもここは Rust から降ってくる形なので、**ずれても型検査は通ります。**
 */
export type DiagEvent = {
	seq: number;
	/** 起動からの経過（ミリ秒）。**画面では秒に直して出します。日付ではありません。** */
	atMs: number;
	level: 'info' | 'warn' | 'error';
	/** どの段か（`Auth` / `Connect` など）。**Rust 側の `Stage` の名前がそのまま来ます。** */
	stage: string;
	/** どの接続の話か。**無い段もあります**（起動そのものの診断など）。 */
	connection?: string;
	message: string;
	/** **どうすれば通るか。**握り潰さないための一言（無いこともあります）。 */
	hint?: string;
};
