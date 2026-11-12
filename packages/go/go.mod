module github.com/dusmamud/dusimagez/go

go 1.22

// No third-party dependencies: the standard library only. The native
// image work lives in the Rust core, reached through cgo (see ffi.go).
// The module carries no version here; releases are tagged vX.Y.Z on
// the repo, in lockstep with the dusimagez core (0.1.0).
