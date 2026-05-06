# Minimal Counter

This example is the smallest end-to-end Cross-Kit app:

- Rust owns `CounterViewModel`, `CounterState`, actions, observer subscription, and metadata.
- iOS uses `CrossKitMinimalCounterBridge(initial:)` and renders `kit.counter.state.value`.
- Android uses `rememberCrossKitMinimalCounterBridge(initial = ...)` and renders `kit.counter.state.value`.

Run the shared tests and metadata binary:

```bash
cargo test -p minimal-counter-shared --lib --tests
cargo run -p minimal-counter-shared --bin ck_minimal_counter_metadata
```

Package and build iOS:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/minimal-counter/cross-kit.toml
xcodebuild -project examples/minimal-counter/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Package and build Android:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/minimal-counter/cross-kit.toml

cd examples/minimal-counter/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug
```
