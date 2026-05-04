# Repository Inventory Notes

This document records the repository state at the start of Step 0 from
`docs/refactor-plan.md`, plus the current mainline locations after later
refactor steps.

## Start State

Command:

```bash
git status --short --branch
```

Observed state:

```text
## main...origin/main [ahead 3]
 M docs/cross-kit-cli.md
 M example/android/app/src/main/java/com/example/crosskit_example_android/MainActivity.kt
 M example/shared/src/lib.rs
 D old-example/Cargo.lock
 D old-example/Cargo.toml
?? crates/ck-vm-macros/Cargo.lock
?? docs/refactor-plan.md
?? example/android/app/src/main/java/com/example/crosskit_example_android/shared/
?? example/android/app/src/main/jniLibs/
```

Notes:

- The working tree was already dirty before Step 0.
- `old-example/Cargo.toml` and `old-example/Cargo.lock` were already deleted before Step 0 started.
- Step 0 does not revert or rewrite unrelated existing changes.

## Step 0 Repository Roles Observed

Main Cross-Kit prototype at the start of Step 0:

- `example/shared`: Rust SDK prototype using UniFFI and VM metadata macros.
- `example/ios`: SwiftUI example consuming generated `CrossKitShared` Swift package.
- `example/android`: Compose example in partial integration state. It references `com.crosskit.shared`, but Kotlin UniFFI bindings and native `.so` are not present in the repository.
- `crates/ck-vm-macros`: current VM metadata macro prototype.
- `tools/ck-swift-packager`: current SwiftPM/XCFramework packaging prototype.

Historical material:

- `old-example`: SigSong historical project and old packaging references.

## Current Mainline Locations

After Step 5, active Cross-Kit example paths are:

- `examples/counter-list/shared`: Rust SDK prototype using UniFFI and the public
  `cross-kit` crate.
- `examples/counter-list/ios`: SwiftUI example consuming the generated
  `CrossKitShared` Swift package from `examples/counter-list/dist/ios`.
- `examples/counter-list/android`: Compose example awaiting the Android
  packager/AAR flow in later steps.
- `crates/cross-kit`: public Rust runtime crate.
- `crates/cross-kit-cli`: `cross-kit` CLI binary.
- `crates/cross-kit-packager-ios`: current SwiftPM/XCFramework packager.

`old-example` remains outside the repository at `../Cross-Kit-old-example` and
is not part of the workspace.

## old-example Inventory

Before migration:

- Approximate size: `3.5G`.
- Git tracked files under `old-example`: `221`.
- Files present under `old-example`: `25062`.
- External migration destination: `../Cross-Kit-old-example`.

Top-level historical components:

| Path | Role | Keep As External Reference |
| --- | --- | --- |
| `old-example/cargo-swift` | Upstream-style `cargo swift` packaging implementation and templates. | Yes, useful for iOS packager behavior and target handling reference. |
| `old-example/sigsong-sdk` | Real Rust + UniFFI SDK with networking, storage, callbacks, Swift/Kotlin binding usage. | Yes, useful as a non-trivial Rust SDK reference. |
| `old-example/sigsong-ios` | SwiftUI app consuming generated Rust SDK package and callback bridge. | Yes, useful as historical iOS integration reference. |
| `old-example/sigsong-android` | Compose app consuming generated Kotlin bindings and native `.so`. | Yes, useful as historical Android integration reference. |

Reference files found before migration:

```text
old-example/README.md
old-example/cargo-swift/Cargo.toml
old-example/cargo-swift/README.md
old-example/sigsong-android/README.md
old-example/sigsong-android/build.gradle.kts
old-example/sigsong-ios/README.md
old-example/sigsong-sdk/Cargo.toml
old-example/sigsong-sdk/README.md
```

## Migration Decision

`old-example` should not remain in this repository and should not enter any workspace. It is moved outside the repository to:

```text
../Cross-Kit-old-example
```

The current repository commit for Step 0 will record `old-example` as removed from the repository. The external directory is an untracked reference backup for humans and future agents.

## Post-Migration Check

After migration:

```text
old-example: absent from repository
../Cross-Kit-old-example: present, 3.5G
```

External backup top-level directories:

```text
../Cross-Kit-old-example/cargo-swift
../Cross-Kit-old-example/sigsong-android
../Cross-Kit-old-example/sigsong-ios
../Cross-Kit-old-example/sigsong-sdk
```

The two root workspace files below were already absent before migration and therefore are not restored inside the repository:

```text
old-example/Cargo.toml
old-example/Cargo.lock
```

They remain recoverable from git history if the external reference needs the original historical workspace root.

## Verification Scope For Step 0

Step 0 should not change Cross-Kit runtime behavior. Verification focuses on:

- Rust shared formatting, tests, and coverage.
- Existing iOS example build.
- Ensuring generated/native artifacts remain untracked.
- Ensuring `old-example` is absent from the repository after migration.

## Step 0 Verification Results

Commands run:

```bash
cargo fmt --manifest-path example/shared/Cargo.toml
cargo test --manifest-path example/shared/Cargo.toml
cargo llvm-cov --manifest-path example/shared/Cargo.toml --summary-only
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
git ls-files | rg '(^|/)(dist|target|DerivedData|\.gradle|build)(/|$)|\.(so|a|dylib|xcframework|aar|apk|ipa|app)$' || true
```

Results:

- `cargo fmt`: passed.
- `cargo test`: passed, 25 library tests and 2 metadata binary tests.
- `cargo llvm-cov`: passed, TOTAL line coverage `97.86%`.
- `xcodebuild build`: passed, `BUILD SUCCEEDED`.
- Generated/native artifact tracking check: passed. The `git ls-files` check returned no tracked `dist`, `target`, `DerivedData`, `.gradle`, `build`, `.so`, `.a`, `.dylib`, `.xcframework`, `.aar`, `.apk`, `.ipa`, or `.app` artifacts.

Test additions:

- No runtime test was added in Step 0 because the step only documents the current state and moves historical material out of the repository.
- Existing runtime tests and coverage remain above the required 97% threshold.

Review notes:

- `cargo fmt` formatted the already-dirty `example/shared/src/lib.rs`. That file contains pre-existing changes from before Step 0 and should not be staged into the Step 0 commit.
