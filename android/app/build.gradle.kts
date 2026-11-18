// Android TV app around the Rust game (ADR 0006). The game itself is `libbattle_tanks.so`, built by the
// `cargoNdk` task below from the repo's Cargo crate; this project only adds the Java GameActivity shell.

plugins {
    id("com.android.application")
}

// Version comes from Cargo.toml so the exe, the APK and the title screen always agree.
val cargoVersion = Regex("""(?m)^version = "(\d+)\.(\d+)\.(\d+)"""")
    .find(rootProject.file("../Cargo.toml").readText())!!.groupValues.drop(1).map { it.toInt() }

android {
    namespace = "com.lunatiq.battletanks"
    compileSdk = 36
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "com.lunatiq.battletanks"
        minSdk = 26
        targetSdk = 36
        versionName = cargoVersion.joinToString(".")
        versionCode = cargoVersion[0] * 10000 + cargoVersion[1] * 100 + cargoVersion[2]
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // ponytail: debug-signed until the Play upload key exists (M6 phase F); then a signingConfig from keystore.properties.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // Must match the GameActivity version android-activity 0.6 was written against (its README).
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
}

// Builds the game for both TV ABIs (many TVs run 32-bit ARM) into jniLibs. Always the Cargo release profile:
// a debug Bevy .so is hundreds of MB and too slow to play.
// `-Pabis=armeabi-v7a` builds one ABI for quick device tests (the owner's Bravia is 32-bit).
val abis = (findProperty("abis") as String? ?: "arm64-v8a,armeabi-v7a").split(",")
val cargoNdk by tasks.registering(Exec::class) {
    workingDir = rootProject.file("..")
    commandLine(
        listOf("cargo", "ndk") + abis.flatMap { listOf("-t", it) } + listOf(
            "-P", "26", "-o", file("src/main/jniLibs").absolutePath,
            "rustc", "--lib", "--release", "--crate-type", "cdylib",
        ),
    )
    doFirst {
        delete("src/main/jniLibs") // no stale ABI left behind by a `-Pabis=` test build
        environment("ANDROID_NDK_HOME", androidComponents.sdkComponents.ndkDirectory.get().asFile.absolutePath)
    }
}
tasks.named("preBuild") { dependsOn(cargoNdk) }
