fn main() {
    // The UDL MUST live in this crate (crates/dusimagez-uniffi/src/),
    // NOT with the core crate. UniFFI derives the FFI symbol namespace
    // from the crate that owns the UDL file, and the compiled scaffolding
    // derives it from this crate's lib name (`uniffi_dusimagez`) — the
    // two must agree, or the generated Kotlin/Swift bindings link
    // against symbols the library never exports (see the Swift link
    // failure on bf03c81: `ffi_dusimagez_*` expected, only
    // `ffi_uniffi_dusimagez_*` present). Do NOT move the UDL back.
    // Path is relative to this crate's manifest dir.
    uniffi::generate_scaffolding("src/dusimagez.udl").unwrap();
}
