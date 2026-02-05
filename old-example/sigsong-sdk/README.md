# SigSong SDK

Rust/UniFFI SDK wrapping SigSong backend APIs with caching, compression, and Swift bindings.

## Capabilities
- HTTP client with automatic Zstandard compression and protobuf envelopes.
- Structured SQLite cache for songs, dictionary entries, recommendations, and the authenticated user.
- Exposed through `InvokeManager` for Swift (via `SigsongSDK` Swift package) and Kotlin (via generated bindings used by the Android app).
- Toast callbacks and document-path delegation back to the host app.

## Public API Highlights
`InvokeManager` now exposes:
- `getSongById`, `getFeedRecommendSongs`, `getWordInfo`, `searchSong`
- `login(username:password:)` – stores `ClUserInfo` and JWT in the encrypted cache.
- `currentUser()` and `logout()` helpers for session management.

The backing cache helpers live under `cache::song`, `cache::word_info`, and `cache::user`.

## Database
Migrations run automatically on start (`sqlx::migrate!`).
- `cached_songs`
- `word_entries`, `word_senses`, `word_meanings`, `word_examples` – normalized storage for dictionary lookups
- `cached_song_recommendations`
- `current_user` stores the latest account profile & token

## Building & Testing
```bash
# Format & compile
cargo fmt
cargo build -p sigsong-sdk

# Library-only tests (network-less)
cargo test -p sigsong-sdk --lib

# Full stack test (requires sigsong-server + databases)
SIGSONG_SERVER_BASE_URL=http://127.0.0.1:7878/api \
    cargo test -p sigsong-sdk --test full_stack
```

## Swift Package Generation
After code changes run:
```bash
cd sigsong-sdk
IPHONEOS_DEPLOYMENT_TARGET=18.0 \
  cargo swift package --name SigsongSDK --lib-type dynamic --platforms ios
```
The resulting Swift package lives in `sigsong-sdk/SigsongSDK`. Copy/sync it to `sigsong-ios/SigSongSDK` (as done in this repo) so Xcode can consume the updated `Package.swift` and framework.
