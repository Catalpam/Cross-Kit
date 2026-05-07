# Search Refresh

This example shows a state-driven long operation rendered through generated iOS
and Android libraries. It intentionally does not expose Rust `async fn`, Swift
`async throws`, or Kotlin `suspend` APIs.

- Rust owns `SearchViewModel`, query validation, loading/progress state,
  cancellation, typed errors, fake result generation, stale-result protection,
  observer subscription, and metadata.
- iOS uses `CrossKitSearchRefreshBridge()` and renders `kit.search.state`.
- Android uses `rememberCrossKitSearchRefreshBridge()` and renders
  `kit.search.state`.
- Platform code only displays state and invokes `updateQuery`, `submit`, `tick`,
  and `cancel`; it does not decide empty-query rules, loading transitions,
  network failure, cancellation state, or result contents.

Run the shared tests and metadata binary:

```bash
cargo test -p search-refresh-shared --lib --tests
cargo run -p search-refresh-shared --bin ck_search_refresh_metadata
```

Package and build iOS:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/search-refresh/cross-kit.toml
xcodebuild -project examples/search-refresh/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Package and build Android:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/search-refresh/cross-kit.toml

cd examples/search-refresh/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug testDebugUnitTest assembleDebugAndroidTest
```
