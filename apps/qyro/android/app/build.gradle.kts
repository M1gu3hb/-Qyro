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
    // CameraX, ADR-0048 y `R10` §1: Jetpack, **cero Play Services**. Un producto
    // cuya primera promesa es "sin nube" no puede depender de una biblioteca de
    // Google Play para mirar por su propia camara.
    //
    // Tres artefactos y ni uno mas: `core` por `ImageProxy` y
    // `ResolutionSelector`, `camera2` por la implementacion, `lifecycle` por
    // `bindToLifecycle`. **No** entra `camera-view`: el `PreviewView` es una
    // vista de Android y esta aplicacion dibuja con Flutter.
    implementation("androidx.camera:camera-core:1.6.1")
    implementation("androidx.camera:camera-camera2:1.6.1")
    implementation("androidx.camera:camera-lifecycle:1.6.1")

    // QYR-0350. Two failures, one cause, and the cause is a floor that is too
    // low rather than a version that is missing.
    //
    // Flutter's `integration_test` plugin declares, in
    // `$FLUTTER/packages/integration_test/android/build.gradle.kts`:
    //     api("androidx.test:runner:1.2+")
    //     api("androidx.test:rules:1.2+")
    //     api("androidx.test.espresso:espresso-core:3.3+")
    // Those are `api` of a library this app depends on, so they land on the
    // app's own `debugRuntimeClasspath`: `1.2+` picks rules 1.2.0, and espresso
    // 3.3.0's hard `runner:1.3.0` lifts runner to 1.3.0.
    //
    // AGP then makes `debugAndroidTestRuntimeClasspath` resolve *consistently
    // with* `debugRuntimeClasspath`, re-emitting every version resolved there as
    // `strictly` on the test classpath. That is where the earlier
    // `runner:{strictly 1.3.0}` came from, and why
    // `androidTestImplementation("androidx.test:runner:1.6.2")` could not work:
    // `require 1.6.2` is the range [1.6.2, inf) and `strictly 1.3.0` is the
    // single point {1.3.0}, so the intersection is empty.
    //
    // The second failure follows from the same floor. `androidx.test:core` is
    // the only artifact in this graph whose AAR declares components: three
    // `InstrumentationActivityInvoker$*` activities with an `<intent-filter>`.
    // Up to and including 1.3.0 it declares them without `android:exported`,
    // which targetSdk 31+ rejects -- that is the
    // `processDebugAndroidTestManifest` error verbatim.
    //
    // So raise the floor where the `strictly` is computed. A constraint is not
    // a dependency: it adds no module to any graph, it only says "if this
    // module appears, it appears at at least this version".
    constraints {
        // `runner` 1.7.0 requires monitor 1.8.0 and services:storage 1.6.0, so
        // monitor rises with it and needs no constraint of its own.
        implementation("androidx.test:runner:1.7.0")
        // Separately, because `rules` 1.2.0 does not follow `runner` upward:
        // conflict resolution would leave it at 1.2.0 and its
        // `{strictly 1.2.0}` would then contradict the declaration below.
        implementation("androidx.test:rules:1.7.0")
        //
        // Deliberately NOT `androidx.test.espresso:espresso-core`. Raising
        // espresso to 3.7.0 also fixes both failures, because espresso 3.7.0
        // requires `androidx.test:core:1.7.0` -- but that drags `core` onto the
        // **application's** debug runtime classpath, and core's AAR manifest
        // also declares
        // `<uses-permission android:name="android.permission.REORDER_TASKS" />`.
        // Measured: the merged debug manifest produced that way carries
        // REORDER_TASKS. That is the APK `platform-builds.yml` installs and
        // asserts on, in a repository whose test is named "the manifest
        // declares exactly one permission". espresso 3.3.0's POM does not
        // depend on `core` at all and nothing here touches Espresso, so it
        // stays where Flutter put it and `core` stays off the app.
    }

    // Everything below carries an explicit version, and each equals the floor
    // above. That is safe by construction, not by luck: the constraint
    // guarantees the app classpath resolves to at least 1.7.0, so the derived
    // `strictly` is at least 1.7.0, and `require 1.7.0` always intersects it.
    // Versionless declarations would also resolve -- but only by borrowing a
    // version from a third-party plugin's dynamic range plus an AGP internal,
    // with nothing underneath if either changes.
    //
    // `core` is named even though `ext:junit:1.3.0` already requires it,
    // because core's manifest **is** the defect this block exists to fix, and a
    // requirement that is only implied cannot be read.
    androidTestImplementation("androidx.test:core:1.7.0")
    // `AndroidJUnit4`, used by KeystoreIdentityTest. 1.3.0 requires core 1.7.0,
    // monitor 1.8.0 and services:storage 1.6.0 -- at or below every floor
    // above, so no `strictly` is violated.
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    // `androidx.test.runner.AndroidJUnitRunner`, the `testInstrumentationRunner`
    // named above; it also brings `androidx.test:monitor`, which supplies the
    // `InstrumentationRegistry` KeystoreIdentityTest uses.
    androidTestImplementation("androidx.test:runner:1.7.0")
    // Unused by this module's tests today, but integration_test's
    // FlutterTestRunner imports `androidx.test.rule.ActivityTestRule`, so the
    // version pinned here has to be one that still has it. 1.7.0 does.
    androidTestImplementation("androidx.test:rules:1.7.0")
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
