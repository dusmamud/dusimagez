// Pre-build check for the WASM playground bundle.
//
// The playground page lazy-loads the WASM build of the Rust core from
// `public/wasm/dusimagez_wasm.js`. That file is produced by
// `wasm-pack build --target web crates/dusimagez-wasm` (see
// `public/wasm/README.md`).
//
// This script NEVER fails the build on purpose: the WASM crate is built by
// another worker, and the docs site must stay deployable while it is still
// in progress. When the WASM build is absent it prints a loud warning and
// the playground page renders its honest "not yet present" fallback instead
// of pretending to work.
import { existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = join(websiteDir, '..');

const wasmJs = join(websiteDir, 'public', 'wasm', 'dusimagez_wasm.js');
const wasmCrate = join(repoRoot, 'crates', 'dusimagez-wasm');

if (existsSync(wasmJs)) {
	console.log('[check-wasm] WASM playground bundle present: public/wasm/dusimagez_wasm.js');
} else if (existsSync(wasmCrate)) {
	console.warn(
		'\n[check-wasm] WARNING: crates/dusimagez-wasm exists but public/wasm/dusimagez_wasm.js is missing.\n' +
			'  Run `wasm-pack build --target web crates/dusimagez-wasm --out-dir website/public/wasm`\n' +
			'  (see public/wasm/README.md).\n' +
			'  The playground will show its "WASM build not yet present" message until then.\n',
	);
} else {
	console.warn(
		'\n[check-wasm] WARNING: no WASM build present (crates/dusimagez-wasm does not exist yet).\n' +
			'  The docs site will still build. The playground page will show its honest\n' +
			'  "WASM build not yet present — run the build step" fallback message.\n',
	);
}
