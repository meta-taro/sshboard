<script lang="ts">
	/**
	 * 細線のインライン SVG。**アイコンのライブラリを足しません。**
	 *
	 * - CSP が `default-src 'self'` で、**デスクトップアプリはオフラインでも動く**
	 * - `currentColor` なので、テーマにも印の色にも自動で乗る
	 * - 線は 1.25px。**太い線のアイコンは、この密度の画面で騒がしくなる**
	 */
	type Name =
		| 'server'
		| 'activity'
		| 'plus'
		| 'check'
		| 'trash'
		| 'sun'
		| 'moon'
		| 'contrast'
		| 'globe'
		| 'key'
		| 'tag'
		| 'user'
		| 'palette'
		| 'warning'
		| 'link'
		| 'folder'
		| 'file'
		| 'upload'
		| 'refresh'
		| 'arrow-up'
		| 'plug'
		| 'unplug'
		| 'copy';

	let { name, size = 14 }: { name: Name; size?: number } = $props();

	const paths: Record<Name, string> = {
		server:
			'M3 5.5h14M3 5.5v3h14v-3M3 11.5h14v3H3zM5.6 7h.01M5.6 13h.01',
		activity: 'M2 10h3.2l2.1-5.4 2.9 10.8 2.2-6.4 1.3 3H18',
		plus: 'M10 4.5v11M4.5 10h11',
		check: 'M4.5 10.5l3.6 3.5L15.5 6.5',
		trash: 'M4 6h12M8 6V4.2h4V6M6 6l.7 9.4h6.6L14 6M8.6 8.6v4.6M11.4 8.6v4.6',
		sun: 'M10 6.6a3.4 3.4 0 100 6.8 3.4 3.4 0 000-6.8zM10 2.4v1.6M10 16v1.6M4.6 4.6l1.1 1.1M14.3 14.3l1.1 1.1M2.4 10H4M16 10h1.6M4.6 15.4l1.1-1.1M14.3 5.7l1.1-1.1',
		moon: 'M15.6 12.3A6.2 6.2 0 017.7 4.4a6.4 6.4 0 100 15.2 6.2 6.2 0 007.9-7.3z',
		contrast: 'M10 2.6a7.4 7.4 0 100 14.8 7.4 7.4 0 000-14.8zM10 2.6v14.8a7.4 7.4 0 000-14.8z',
		globe: 'M10 2.6a7.4 7.4 0 100 14.8 7.4 7.4 0 000-14.8zM2.9 10h14.2M10 2.6c1.9 2 2.9 4.6 2.9 7.4s-1 5.4-2.9 7.4c-1.9-2-2.9-4.6-2.9-7.4s1-5.4 2.9-7.4z',
		key: 'M12.6 3.6a4 4 0 00-3.4 6.1L3.4 15.5v1.9h1.9l.9-.9v-1.3h1.3l.9-.9v-1.3h1.3l1-1a4 4 0 101.9-7.6zM13.5 6.9h.01',
		tag: 'M3.6 3.6h5.2l7.6 7.6-5.2 5.2L3.6 8.8zM6.4 6.4h.01',
		user: 'M10 3.4a3 3 0 100 6 3 3 0 000-6zM4 16.6c0-2.6 2.7-4.4 6-4.4s6 1.8 6 4.4',
		palette:
			'M10 2.6c-4.1 0-7.4 3.1-7.4 7 0 3.5 2.5 5.1 4.6 5.1 1.4 0 1.9-.9 1.9-1.7 0-1.1-.9-1.3-.9-2.2 0-.8.7-1.5 1.7-1.5h2.2c2.1 0 3.3-1.4 3.3-3.2 0-2.1-2.3-3.5-5.4-3.5zM6.3 8.3h.01M9 5.9h.01M12.4 6.8h.01',
		warning: 'M10 3.6L2.8 16.4h14.4L10 3.6zM10 8.4v3.6M10 14h.01',
		link: 'M8.4 11.6a3 3 0 004.5.3l2.2-2.2a3 3 0 10-4.2-4.2l-1.2 1.2M11.6 8.4a3 3 0 00-4.5-.3L4.9 10.3a3 3 0 104.2 4.2l1.2-1.2',
		folder: 'M2.8 5.2h4.4l1.4 1.8h6.6a1 1 0 011 1v6.4a1 1 0 01-1 1H2.8a1 1 0 01-1-1V6.2a1 1 0 011-1z',
		file: 'M5.2 2.8h6l3.6 3.6v10.8H5.2zM11.2 2.8v3.6h3.6',
		upload: 'M10 14.4V4.6M6.4 8.2L10 4.6l3.6 3.6M3.4 16.4h13.2',
		refresh: 'M16.2 8.4A6.4 6.4 0 004.6 6.2M3.8 11.6a6.4 6.4 0 0011.6 2.2M4.4 3.4v2.8h2.8M15.6 16.6v-2.8h-2.8',
		'arrow-up': 'M10 16V4.4M5.4 9L10 4.4 14.6 9',
		plug: 'M7 2.8v4M13 2.8v4M4.6 6.8h10.8v2.6a5.4 5.4 0 01-10.8 0zM10 14.8v3.4',
		unplug: 'M4.6 6.8h10.8v2.6a5.4 5.4 0 01-10.8 0zM10 14.8v3.4M3 3l14 14',
		copy: 'M7.4 7.4h8.2v8.2H7.4zM4.4 12.6V4.4h8.2'
	};
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 20 20"
	fill="none"
	stroke="currentColor"
	stroke-width="1.25"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	focusable="false"
	class="icon"
>
	<path d={paths[name]} />
</svg>

<style>
	.icon {
		flex: none;
		display: block;
	}
</style>
