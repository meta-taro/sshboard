/**
 * 画面を**画面側で**描いて 1 枚にする（D36）。
 *
 * **OS の画面収録に頼りません。**
 *
 * これまでは OS の画面キャプチャ（`xcap`）で撮っていましたが、
 * **macOS は「画面収録」の許可が要り、しかも許可はバイナリに紐づきます** —
 * 開発中は再ビルドのたびに外れ、配布後も人が 1 回許すまで撮れません。
 * **事象をサポートへ送るための機能が、許可の可否で使えなくなる**のは本末転倒です。
 *
 * この道具の画面は WebView の中の DOM です。**xterm も DOM で描いています**
 * （WebGL / Canvas のアドオンを入れていない）ので、
 * **端末を含めて、そのまま描き直せます。**
 *
 * ## 伏せる順序は変えていません
 *
 * 撮る前に Rust 側が「伏せろ」と言い、伏せ終わってからここが呼ばれます（D26）。
 * **伏せていない画素は最初から作られません。**
 *
 * ## OS のキャプチャと何が違うか
 *
 * ここが描くのは **DOM をもう一度描いたもの**で、画面に出ている実物ではありません。
 * 崩れ（重なり・はみ出し・潰れ）は同じ CSS で描くので**そのまま出ます**が、
 * WebView 自身の描画不具合や、上に乗った OS のダイアログは写りません。
 * **写らないことを承知で、常に撮れる方を既定にしています。**
 */

import { emit, listen } from '@tauri-apps/api/event';

/** Rust が「描いてくれ」と言ってくる。 */
const REQUEST = 'capture://page';
/** 描けた／描けなかったを返す。 */
const RESULT = 'capture://page-result';

type Request = { maxEdge: number };

type Result =
	| { ok: true; png: string; width: number; height: number; scaledWidth: number; scaledHeight: number }
	| { ok: false; error: string };

/**
 * 受け口を開く。**起動時に 1 回だけ呼びます。**
 *
 * Tauri の外（素の `vite dev`）では何もしません。
 */
export async function listenForPageShot(): Promise<() => void> {
	if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
		return () => {};
	}
	return listen<Request>(REQUEST, async (event) => {
		try {
			const shot = await withLimit(draw(event.payload?.maxEdge ?? 1600));
			await emit(RESULT, shot);
		} catch (error: unknown) {
			// **黙らない。**描けなかったことが分からないと、Rust 側は待ち続けます。
			await emit(RESULT, { ok: false, error: String(error) } satisfies Result);
		}
	});
}

/**
 * 描画そのものに時間切れを設ける。
 *
 * **止まったまま返らないのが一番困ります。**Rust 側は待ち続け、
 * 人には「20 秒以内に描き終わりませんでした」としか出ません。
 * ここで切れば、**何をしている最中に止まったか**が返せます。
 */
async function withLimit(work: Promise<Result>): Promise<Result> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	const limit = new Promise<Result>((resolve) => {
		timer = setTimeout(
			() => resolve({ ok: false, error: '描画が 12 秒で終わりませんでした' }),
			12_000
		);
	});
	try {
		return await Promise.race([work, limit]);
	} finally {
		if (timer !== undefined) clearTimeout(timer);
	}
}

async function draw(maxEdge: number): Promise<Result> {
	const { domToPng } = await import('modern-screenshot');

	const root = document.documentElement;
	const width = root.clientWidth;
	const height = root.clientHeight;
	if (width === 0 || height === 0) {
		return { ok: false, error: 'ウィンドウの大きさが 0 です（最小化されていませんか）' };
	}

	// **引き伸ばしません**（D26）。小さい窓を大きく返しても崩れは見やすくなりません。
	const longest = Math.max(width, height);
	const scale = longest > maxEdge ? maxEdge / longest : 1;

	const url = await domToPng(root, {
		// **`width` / `height` は渡しません。**`scale` と併せると
		// 出力の座標がずれ、地の色だけの真っ白が返りました（実測）。
		scale,
		// **外へ取りに行かない。**この道具はオフラインでも動きます（CSP も `self` のみ）。
		// 取りに行くと、返らないまま止まります。
		font: false,
		// 1 つ読めないものがあっても、**全部を諦めない。**
		timeout: 8_000,
		// **地を必ず塗る。**透明のまま返すと、見る側の背景で色が変わって
		// 「暗いはずが明るい」といった誤読を生みます。
		backgroundColor: getComputedStyle(root).getPropertyValue('--ground')?.trim() || '#ffffff'
	});

	const comma = url.indexOf(',');
	if (!url.startsWith('data:image/png;base64,') || comma < 0) {
		return { ok: false, error: '描けたものが PNG ではありませんでした' };
	}

	return {
		ok: true,
		png: url.slice(comma + 1),
		width,
		height,
		scaledWidth: Math.round(width * scale),
		scaledHeight: Math.round(height * scale)
	};
}
