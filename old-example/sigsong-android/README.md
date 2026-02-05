# SigSong Android

Kotlin + Jetpack Compose client that consumes the shared UniFFI SDK. The app mirrors the iOS feature set:

- Login flow backed by the Rust SDK (demo credentials `demo@sigsong` / `sigsong123`).
- Recommendation feed with pagination.
- Search powered by the `SearchSong` API.
- Lyrics detail screen with quick word-info lookups (first matching sense shown via snackbar).
- Toast notifications propagated from Rust via the UniFFI callback interface.

## Project layout

```
app/
  src/main/java/com/sigsong/android   ← Compose UI & platform glue
  src/main/java/com/sigsong/sdk/...   ← Generated Kotlin bindings (`uniffi`) 
  src/main/jniLibs/                  ← Prebuilt `libsig_song_sdk.so`
```

## Updating the Rust bindings

1. Build native libraries for the desired ABIs (example: `arm64-v8a` & `x86_64`):
   ```bash
   cd sigsong-sdk
   cargo ndk -t arm64-v8a -t x86_64 -o ../target/android build --release
   cp ../target/android/arm64-v8a/libsig_song_sdk.so ../sigsong-android/app/src/main/jniLibs/arm64-v8a/
   cp ../target/android/x86_64/libsig_song_sdk.so ../sigsong-android/app/src/main/jniLibs/x86_64/
   ```
2. Regenerate Kotlin bindings from the compiled cdylib:
   ```bash
   cargo run --features binding-generator --bin uniffi-bindgen \
     -- ../target/android/arm64-v8a/libsig_song_sdk.so \
        ../sigsong-android/app/src/main/java/com/sigsong/sdk
   ```
   (The binding generator writes into `com/sigsong/sdk/uniffi/sig_song_sdk/`.)

## Building

Requirements:
- JDK 21 (`brew install openjdk@21` on macOS, set `$JAVA_HOME` accordingly)
- Android SDK/NDK (paths inherited from the global environment used by the Rust SDK build)

Build command:
```bash
cd sigsong-android
JAVA_HOME=/path/to/jdk ./gradlew assembleDebug
```
If the environment blocks downloads from `services.gradle.org`, point `gradle-wrapper.properties` to a locally cached Gradle distribution (e.g. `distributionUrl=file:/path/to/gradle-8.5-bin.zip`).

The resulting APK lives in `app/build/outputs/apk/debug/`.

## Known limitations
- First `./gradlew` invocation requires internet access to fetch the Android Gradle Plugin. In restricted environments, fetch the artifacts manually and place them in a local Maven mirror.
- The UI currently surfaces word-info summaries via snackbars. A richer dictionary sheet can be layered on top of the structured cache if needed.

