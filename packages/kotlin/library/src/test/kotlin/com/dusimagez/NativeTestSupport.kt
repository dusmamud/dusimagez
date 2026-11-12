package com.dusimagez

import org.junit.Assume.assumeTrue

/**
 * Gate for tests that need the real Rust core. The native library
 * (`libuniffi_dusimagez.so`) is built by CI from the core crate and
 * pointed at with `DUSIMAGEZ_NATIVE_DIR` (see library/build.gradle.kts).
 * When it is absent (e.g. a local checkout without a cargo build) the
 * native tests are SKIPPED via JUnit assumptions — reported as
 * skipped, never as passed. Nothing here fakes a pass.
 */
object NativeTestSupport {

    @Volatile
    private var checked = false

    @Volatile
    private var available = false

    /** Returns true when the native library loaded; assumes (skips) otherwise. */
    fun assumeAvailable() {
        if (!checked) {
            available = try {
                uniffi.dusimagez.uniffiEnsureInitialized()
                true
            } catch (e: Throwable) {
                System.err.println(
                    "dusimagez native library not available, skipping native test: $e " +
                        "(build the core cdylib and set DUSIMAGEZ_NATIVE_DIR)",
                )
                false
            }
            checked = true
        }
        assumeTrue("native libuniffi_dusimagez not available", available)
    }
}
