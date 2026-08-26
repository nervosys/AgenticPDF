// SPDX-License-Identifier: AGPL-3.0-or-later
plugins {
    id("com.android.application") version "8.7.0"
    kotlin("android") version "2.0.20"
}

android {
    namespace = "ai.nervosys.apdf"
    compileSdk = 35

    defaultConfig {
        applicationId = "ai.nervosys.apdf"
        // 21 matches the NDK linker wrappers configured in .cargo/config.toml.
        minSdk = 21
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
    }

    // Cargo has already produced the .so files; Gradle only packages them.
    // Keeping the Rust build out of Gradle means `cargo build` stays the single
    // way to build the core, and the APK cannot embed a stale library that
    // Gradle rebuilt differently.
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    kotlinOptions { jvmTarget = "17" }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
