# Task Board

This example shows a Rust-owned task board rendered through generated iOS and
Android libraries:

- Rust owns `TaskBoardViewModel`, `TaskListViewModel`, filters, counters,
  ordering, list diffs, observer subscription, and metadata.
- iOS uses `CrossKitTaskBoardBridge()` and renders `kit.taskBoard.state` plus
  `kit.taskList.items`.
- Android uses `rememberCrossKitTaskBoardBridge()` and renders
  `kit.taskBoard.state` plus `kit.taskList.items`.
- Platform code only displays state/items and invokes actions such as
  `addTask`, `renameTask`, `toggleDone`, `moveVisible`, and `setFilter`; it does not
  recalculate filters, counters, or list diffs.

Run the shared tests and metadata binary:

```bash
cargo test -p task-board-shared --lib --tests
cargo run -p task-board-shared --bin ck_task_board_metadata
```

Package and build iOS:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/task-board/cross-kit.toml
xcodebuild -project examples/task-board/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Package and build Android:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/task-board/cross-kit.toml

cd examples/task-board/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug testDebugUnitTest assembleDebugAndroidTest
```
