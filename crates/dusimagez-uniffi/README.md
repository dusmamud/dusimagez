# dusimagez-uniffi

UniFFI scaffolding for dusimagez — the `cdylib` that Kotlin (Android)
and Swift (iOS/macOS) bind against. **No logic lives here**: every type
is a thin adapter over the `dusimagez` core crate, and every function
converts its arguments and delegates to the core.

## Layout

- `src/lib.rs` — adapter types (names/fields match the UDL exactly),
  conversions to/from core types, and the five SPEC §1 functions.
- `src/dusimagez.udl` lives with the core crate
  (`crates/dusimagez/src/dusimagez.udl`) — it is the shared contract
  for the Kotlin/Swift bindings, and `build.rs` generates the
  scaffolding from that same file.
- `[lib] name = "uniffi_dusimagez"`, `crate-type = ["cdylib", "staticlib", "rlib"]`.
  The generated Kotlin loads the library as `uniffi_dusimagez`
  (JNA); do not rename it. The Swift side links the same symbols
  via the `DusimagezFFI` modulemap.

## UDL divergences (see the UDL header for full notes)

- `CompressOptions.dpi`: UDL `Dpi?` record <-> core `Option<(u32, u32)>`.
- `CompressOptions.subsampling`: UDL `null` means "let the core apply
  the quality-dependent rule" — the adapter passes the core's
  not-explicit sentinel (`S420`), resolved to S420 for quality < 90
  and S444 otherwise.
- `ImageInfo.format` / `colour_type`: UDL strings, lowercase — the
  adapter lowercases the core's `Debug` colour-type name
  (`"Rgb8"` -> `"rgb8"`) and uses the format's lowercase extension.
- `bulk_compress` never throws: per-item outcomes ride in
  `BulkItemResult`.

## Build

```bash
cargo build -p dusimagez-uniffi        # libuniffi_dusimagez.so/.rlib
cargo build --release -p dusimagez-uniffi
```

Regenerate the Kotlin/Swift bindings after any UDL change:

```bash
packages/kotlin/scripts/regenerate-bindings.sh
packages/swift/scripts/regenerate-bindings.sh
```

Both scripts use `uniffi-bindgen 0.29` against
`crates/dusimagez/src/dusimagez.udl`.
