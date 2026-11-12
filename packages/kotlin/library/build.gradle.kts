plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.dusimagez"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        // UniFFI needs JNI; the native library is built from the Rust
        // core (see README.md). The AAR ships the .so per ABI; local
        // JVM unit tests load it via jna.library.path (see below).
        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86", "x86_64")
        }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    // UniFFI-generated Kotlin needs JNA at runtime. Local JVM unit
    // tests run on the host JVM, so they need the host-built native
    // library: CI builds it with cargo (`cargo build --release -p
    // dusimagez-uniffi`, cdylib) and points DUSIMAGEZ_NATIVE_DIR at
    // the directory holding libuniffi_dusimagez.so /
    // libuniffi_dusimagez.dylib. Without it the native tests skip
    // via JUnit assumptions (see NativeTestSupport) — reported as
    // skipped, never as passed.
    tasks.withType<Test> {
        val nativeDir = System.getenv("DUSIMAGEZ_NATIVE_DIR")
        if (nativeDir != null) {
            systemProperty("jna.library.path", nativeDir)
        }
        // Where to find shared/test-vectors.json (repo root). CI
        // checks out the whole repo, so the default (walk up from the
        // module dir) works; override only for exotic layouts.
        systemProperty("dusimagez.repo.root", System.getenv("DUSIMAGEZ_REPO_ROOT") ?: "")
    }
}

dependencies {
    // UniFFI Kotlin bindings are built on JNA.
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    testImplementation("junit:junit:4.13.2")
    testImplementation("net.java.dev.jna:jna:5.14.0")
}
