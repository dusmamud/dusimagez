fn main() {
    // The UDL lives with the core crate (crates/dusimagez/src/); the
    // generated Kotlin/Swift bindings are built from that same file.
    // Path is relative to this crate's manifest dir.
    uniffi::generate_scaffolding("../dusimagez/src/dusimagez.udl").unwrap();
}
