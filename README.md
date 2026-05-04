# Cross-Kit

Cross-Kit is a Rust-first toolkit for building shared state-model SDKs and
packaging them as platform libraries that iOS and Android apps can depend on
directly.

Current workspace roles:

- `crates/cross-kit`: public Rust runtime crate for SDK authors.
- `crates/cross-kit-cli`: `cross-kit` CLI binary for metadata and packaging
  workflows.
- `crates/cross-kit-codegen`: Swift/Kotlin bridge source generation.
- `crates/cross-kit-core`: shared config and metadata contracts.
- `crates/cross-kit-packager-ios`: iOS SwiftPM/XCFramework packager.
- `examples/counter-list`: end-to-end sample Rust SDK, iOS app, and Android
  app.

The iOS example consumes generated output from the CLI:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Generated `dist/` directories are ignored by git and should be recreated from
source.

The Android example consumes the generated local Maven artifact:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 cargo run -p cross-kit-cli -- android package --config examples/counter-list/cross-kit.toml
cd examples/counter-list/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug
```
