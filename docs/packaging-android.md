# Android Packaging

Step 7 packages the counter-list shared crate as an Android AAR and publishes
it to an ignored local Maven repository that app code can depend on directly.

Build the AAR:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 cargo run -p cross-kit-cli -- android package --config examples/counter-list/cross-kit.toml
```

Then build the app, which consumes `com.crosskit:crosskitshared:0.1.0` from
the generated local Maven repository:

```bash
cd examples/counter-list/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug
```

The package command writes ignored outputs:

```text
examples/counter-list/dist/android/crosskitshared-release.aar
examples/counter-list/dist/android/maven/
examples/counter-list/dist/android/gradle-project/
examples/counter-list/dist/android/native/
```

For Step 6 debugging, the generated-source flow is still available:

```bash
cargo run -p cross-kit-cli -- gen bridges --platform android --config examples/counter-list/cross-kit.toml
cargo run -p cross-kit-cli -- android build-native --config examples/counter-list/cross-kit.toml
```

The app imports `com.crosskit.shared.*` bridge APIs. It does not call
`System.loadLibrary`, manage `jniLibs`, or compile UniFFI generated sources
directly. Runtime dependencies such as JNA are declared by the generated Maven
POM, not by the app.

Required local tools:

- JDK 17 or newer. This workspace has been verified with Homebrew OpenJDK 21.
- Android SDK and NDK installed under `~/Library/Android/sdk`.
- Rust Android targets for the configured ABIs.
- `cargo-ndk`.
