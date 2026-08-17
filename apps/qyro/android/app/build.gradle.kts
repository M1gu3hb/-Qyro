import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// The release signing material, if this machine has it.
//
// Phase 08. `key.properties` is **not** in the repository and never will be: a
// signing key in a public repository is not a signing key. What is committed is
// `key.properties.example` and the certificate's SHA-256 in
// `docs/release/v1.0.md`, so anybody can check that an APK they were given was
// signed by the key that document names.
//
// Without the file the build signs with the debug key and says so, rather than
// failing: a contributor should be able to build and run.
val signingProperties = Properties().apply {
    val file = rootProject.file("key.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}

android {
    namespace = "com.owner.qyro"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // Phase 08. The Kotlin package stays `com.owner.qyro` because moving it
        // renames every file for no behavioural gain; the *application id* is
        // what the world sees, and `owner` was a placeholder that must not ship.
        // `dev.qyro.app` matches the platform channels this app already uses
        // (`dev.qyro/file_picker`, `dev.qyro/discovery`).
        //
        // This id is permanent: on Android it is the identity of the
        // application, and changing it after a release means every installed
        // copy is a different app that cannot be updated.
        applicationId = "dev.qyro.app"
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        // Phase 06, QYR-0064. The Keystore evidence runs **inside an
        // application process**, under `am instrument`, because a binary pushed
        // to /data/local/tmp has no JVM, no Context and no application process
        // and therefore cannot reach Keystore at all.
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        create("release") {
            val store = signingProperties.getProperty("storeFile")
            if (store != null) {
                storeFile = file(store)
                storePassword = signingProperties.getProperty("storePassword")
                keyAlias = signingProperties.getProperty("keyAlias")
                keyPassword = signingProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            signingConfig = if (signingProperties.getProperty("storeFile") != null) {
                signingConfigs.getByName("release")
            } else {
                // Debug keys, and the build log says so. Silently shipping a
                // debug-signed release is how an unsignable update happens
                // later.
                logger.lifecycle(
                    "qyro: no key.properties; signing the release build with the " +
                        "debug key. See docs/release/v1.0.md.",
                )
                signingConfigs.getByName("debug")
            }
        }
    }

    sourceSets {
        getByName("androidTest") {
            java.srcDirs("src/androidTest/kotlin")
        }
    }
}

dependencies {
    // QYR-0350. These carried versions -- 1.2.1, 1.6.2, 1.6.1 -- and the build
    // failed at `:app:mergeDebugAndroidTestAssets` with "Cannot find a version
    // of 'androidx.test:runner' that satisfies the version constraints".
    //
    // The cause is not a missing artifact. Flutter's `integration_test` plugin
    // declares `api("androidx.test:runner:1.2+")` and
    // `api("androidx.test.espresso:espresso-core:3.3+")`, which land in
    // `debugRuntimeClasspath` and resolve to 1.3.0; AGP's **consistent
    // resolution** then re-imposes that as `strictly 1.3.0` on the androidTest
    // classpath. A newer version declared here cannot satisfy a `strictly`, so
    // naming one is naming a conflict.
    //
    // Declared without a version on purpose: the constraint supplies it, so
    // this follows whatever `integration_test` resolves to instead of racing
    // it on every Flutter upgrade. `androidx.test.ext:junit` is the only one
    // that needs a version, because nothing else puts it on any classpath, and
    // 1.1.2 is the release that pairs with runner 1.3.0.
    androidTestImplementation("androidx.test.ext:junit:1.1.2")
    androidTestImplementation("androidx.test:runner")
    androidTestImplementation("androidx.test:rules")
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
