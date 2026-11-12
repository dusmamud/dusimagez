# dusimagez Go binding

.PHONY: ffi build test vet fmt clean

# Build the native library, then the Go package.
ffi:
	bash scripts/build-ffi.sh

build: ffi
	CGO_ENABLED=1 go build ./...

# go test needs the native library (build-ffi.sh) and, for the shared
# vectors, the `dusimagez mock` CLI (`cargo build -p dusimagez-cli`
# --release) or $DUSIMAGEZ_CLI. Without the CLI the vector fixtures
# are generated in-test and size-bound assertions are skipped (logged).
test: ffi
	CGO_ENABLED=1 go test ./...

vet: ffi
	CGO_ENABLED=1 go vet ./...

fmt:
	gofmt -w -l .

clean:
	rm -f ../../target/release/libdusimagez_ffi.a
