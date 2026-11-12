# dusimagez website

Public docs + landing + WASM playground for dusimagez. Astro 5 + Starlight,
fully static, deployed to Cloudflare Pages at
[https://dusimagez.pages.dev/](https://dusimagez.pages.dev/).

## Develop

```bash
npm ci
npm run dev
```

## Build

```bash
npm run build   # runs scripts/check-wasm.mjs first, then astro build
```

## WASM playground

The playground page (`src/pages/playground.astro`) lazy-loads the Rust core
compiled to WebAssembly from `public/wasm/dusimagez_wasm.js`.

That file is built by the WASM worker from `crates/dusimagez-wasm`
with the repo script (a plain `wasm-pack build` fails at the link
step — C libwebp needs the wasi-shim libc symbols):

```bash
crates/dusimagez-wasm/scripts/wasm-build.sh web website/public/wasm
```

`public/wasm/` is gitignored except for its README — the built bundle is a
build artefact, not source. While the WASM build is absent, `npm run build`
still succeeds (the check script only warns) and the playground renders an
honest "WASM build not yet present — run the build step" message instead of
pretending to work.

## Deploy

Cloudflare Pages, via `.github/workflows/website.yml` (repo root). Needs the
`CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID` repo secrets — a dashboard
step, not code.
