# iOS Packaging

Step 4 introduces the first supported CLI path for generating the iOS端上库 from a Rust SDK crate.

Current example path:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
```

The CLI reads `cross-kit.toml`, compiles the Rust shared crate for the configured Apple targets, runs UniFFI Swift binding generation, asks `cross-kit-codegen` for Swift bridge files, and writes a Swift Package under the configured iOS output directory.

For the current `examples/counter-list` layout, the output is:

```text
examples/counter-list/dist/ios/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/
    cross_kit_shared.swift
    Bridges/*.swift
  cross_kit_sharedFFI.xcframework
```

`examples/counter-list/ios` depends on that generated Swift Package through a local SwiftPM reference:

```text
../dist/ios/CrossKitShared
```

The generated `dist/` directory is ignored by git. It must be recreated from source before opening or building the iOS example:

```bash
rm -rf examples/counter-list/dist
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Configuration fields used by Step 4:

```toml
[shared]
crate_path = "shared"
package = "shared"
lib_name = "cross_kit_shared"
metadata_bin = "ck_vm_metadata"

[ios]
package_name = "CrossKitShared"
output = "dist/ios"
targets = ["ios", "ios-sim", "ios-sim-x86_64"]
build_mode = "release"
lib_type = "static"
format = "spm"
swift_bridges = true
```

Paths are resolved relative to the config file, so the example config keeps
generated output under `examples/counter-list/dist/ios`.
