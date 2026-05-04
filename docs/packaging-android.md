# Android Generation

Step 6 supports the pre-AAR Android flow for the counter-list example.

Generate Compose bridge sources:

```bash
cargo run -p cross-kit-cli -- gen bridges --platform android --config examples/counter-list/cross-kit.toml
```

Build Android native libraries and UniFFI Kotlin bindings:

```bash
cargo run -p cross-kit-cli -- android build-native --config examples/counter-list/cross-kit.toml
```

Then build the app:

```bash
cd examples/counter-list/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew assembleDebug
```

Generated files are written under ignored paths:

```text
examples/counter-list/android/app/build/generated/cross-kit/uniffi/
examples/counter-list/android/app/build/generated/cross-kit/bridges/
examples/counter-list/android/app/src/main/jniLibs/
```

The app imports `com.crosskit.shared.*` bridge APIs. It does not call
`System.loadLibrary` or manage UniFFI low-level bindings directly.

Required local tools:

- JDK 17 or newer. This workspace has been verified with Homebrew OpenJDK 21.
- Android SDK and NDK installed under `~/Library/Android/sdk`.
- Rust Android targets for the configured ABIs.
- `cargo-ndk`.
