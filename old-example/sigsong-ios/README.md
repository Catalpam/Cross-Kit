# SigSong iOS App

SwiftUI client integrating the Rust SDK through the locally generated `SigsongSDK` Swift package.

## Screens
- **Home** – Infinite recommendations, pull-to-refresh, and search entry. Search leverages the new `searchSong` API and displays live results with graceful loading & error states.
- **Lyrics** – Detailed lyrics view backed by cached song data and word-info lookups.
- **Login** – Validates the demo credentials via `API.login`, persists the session in the SDK cache, and dismisses the login flow on success.

## Dependencies
- `SigsongSDK` (local Swift package produced by `cargo swift package`)
- `InvokeKit` (bridges SDK callbacks such as toast notifications)
- `SDWebImageSwiftUI`, `SFSafeSymbols`

Make sure `SigSongSDK/Package.swift` is refreshed whenever the Rust SDK changes. The package is referenced as a local SwiftPM dependency.

## Building
```bash
cd sigsong-ios
xcodebuild \
  -project SigSong.xcodeproj \
  -scheme SigSong \
  -configuration Debug \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  build
```
This assumes you regenerated the Swift package (`IPHONEOS_DEPLOYMENT_TARGET=18.0 cargo swift package ...`) before invoking Xcode.

## Login Credentials
The current backend accepts the demo account `demo@sigsong` with password `sigsong123`. Successful login stores the user profile (`ClUserInfo`) and JWT locally via the SDK.

