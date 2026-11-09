// 配布ページの「どの資産がどの OS・CPU 向けか」の見分け（site/releases.js）。
// node --test tools/site-releases.test.mjs で走ります（依存なし）。
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { sortAssets, published, macLabel } = require('../site/releases.js');

const asset = (name) => ({ name, size: 10485760, browser_download_url: `https://example.invalid/${name}` });

test('every installer lands in its own slot, and x64 and ARM64 are never swapped', () => {
	const got = sortAssets([
		asset('sshboard_9.9.9_arm64-setup.exe'),
		asset('sshboard_9.9.9_x64-setup.exe'),
		asset('sshboard_9.9.9_arm64_en-US.msi'),
		asset('sshboard_9.9.9_x64_en-US.msi'),
		asset('sshboard_9.9.9_universal.dmg'),
		asset('sshboard.app.tar.gz'),
		asset('latest.json')
	]);
	assert.equal(got.winX64.name, 'sshboard_9.9.9_x64-setup.exe');
	assert.equal(got.winArm.name, 'sshboard_9.9.9_arm64-setup.exe');
	assert.equal(got.msiX64.name, 'sshboard_9.9.9_x64_en-US.msi');
	assert.equal(got.msiArm.name, 'sshboard_9.9.9_arm64_en-US.msi');
	assert.equal(got.mac.name, 'sshboard_9.9.9_universal.dmg');
});

test('the dmg is preferred over the updater archive, which people should not download', () => {
	const got = sortAssets([asset('sshboard.app.tar.gz'), asset('sshboard_9.9.9_aarch64.dmg')]);
	assert.equal(got.mac.name, 'sshboard_9.9.9_aarch64.dmg');
	// 古い版には dmg が無い —— そのときだけ従来の形へ落ちる。
	assert.equal(sortAssets([asset('sshboard.app.tar.gz')]).mac.name, 'sshboard.app.tar.gz');
});

test('a mac build says which Macs it runs on', () => {
	assert.equal(macLabel('sshboard_9.9.9_universal.dmg'), 'universal');
	assert.equal(macLabel('sshboard_9.9.9_aarch64.dmg'), 'apple-silicon');
	assert.equal(macLabel('sshboard_9.9.9_x64.dmg'), 'intel');
});

test('drafts and the updater tag are not versions', () => {
	const list = [
		{ tag_name: 'updater', draft: false },
		{ tag_name: 'v9.9.9', draft: true },
		{ tag_name: 'v9.9.8', draft: false },
		{ tag_name: 'v9.9.7', draft: false }
	];
	assert.deepEqual(
		published(list).map((r) => r.tag_name),
		['v9.9.8', 'v9.9.7']
	);
});
