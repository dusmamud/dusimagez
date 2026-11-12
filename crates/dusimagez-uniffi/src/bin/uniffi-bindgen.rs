// `uniffi-bindgen` binary for this crate (UniFFI 0.29).
//
// The `uniffi_bindgen` crate ships no binary of its own in 0.29, and
// there is no matching `uniffi-bindgen` binary crate on crates.io —
// so this workspace provides one via `uniffi::uniffi_bindgen_main()`
// (feature `cli`). It always matches the `uniffi` version this crate
// is built against; never install a separate `uniffi-bindgen` for
// this crate.
//
// Regenerate the Kotlin bindings:
//   cargo run -p dusimagez-uniffi --bin uniffi-bindgen -- generate \
//     --language kotlin --out-dir <tmp> \
//     crates/dusimagez-uniffi/src/dusimagez.udl
// (see packages/kotlin/scripts/regenerate-bindings.sh; the Swift
// script runs the same command with `--language swift`).

fn main() {
    uniffi::uniffi_bindgen_main()
}
