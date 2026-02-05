# SigSong

Multi-component workspace for the SigSong backend, Rust SDK, and iOS client.

## 目录概览

| 目录 | 简介 |
| --- | --- |
| [`sigsong-server`](sigsong-server/README.md) | Axum/Tokio 后端，提供歌曲、词典、搜索、登录等接口 |
| [`sigsong-sdk`](sigsong-sdk/README.md) | Rust + UniFFI SDK，负责网络访问、Zstd 压缩、SQLite 缓存与登录状态存储 |
| [`sigsong-ios`](sigsong-ios/README.md) | SwiftUI 客户端，使用本地生成的 `SigsongSDK` Swift 包 |
| `sigsong-pb` | 公用 protobuf schema |
| [`sigsong-android`](sigsong-android/README.md) | Kotlin + Compose Android 客户端，使用同一套 UniFFI SDK |

详细开发说明请参考各目录下的 README。

## 新增能力速览
- `SearchSong` 接口完成端到端实现（服务端 → SDK → iOS 与 Android 搜索界面），并对 Mongo 读取异常做降级处理。
- `Login` 接口打通：服务器签发 JWT，SDK 将 `ClUserInfo` + Token 持久化到 `current_user` 表，iOS/Android 均可直接复用。
- 词典结果改为结构化存储（`word_entries` / `word_senses` 等表），支持离线缓存、注音与例句。
- 新增 Android Compose 客户端（登录、推荐、搜索、歌词详情）并通过同一套 UniFFI Kotlin 绑定调用 SDK。
- 后端与 SDK 模块布局迁移至 Rust 2018 推荐的 “无 mod.rs” 结构。

## 数据源
- MongoDB：`song` 数据库，用于歌曲与推荐。
- PostgreSQL：`dictionary` 数据库，用于词典检索。示例建表语句及 Compose 配置参见旧 README 内容。

## 常用环境变量
`sigsong-server`：
| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `SIGSONG_SERVER_ADDR` | `0.0.0.0:7878` | HTTP 监听地址 |
| `SIGSONG_MONGO_URI` | `mongodb://localhost:27017/` | Mongo 连接串 |
| `SIGSONG_MONGO_DATABASE` | `song` | Mongo 数据库 |
| `SIGSONG_MONGO_COLLECTION` | `song` | Mongo 集合 |
| `SIGSONG_POSTGRES_URL` | `postgres://postgres:sigsong_pg@localhost:5432/dictionary` | PostgreSQL 连接串 |

`sigsong-sdk`：
| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `SIGSONG_SERVER_BASE_URL` | `http://127.0.0.1:7878/api` | 统一网关入口 |
| `SIGSONG_TEST_WORD` | `描いた` | 集成测试使用的词条 |
| `SIGSONG_TEST_SONG_ID` | `61301499` | 集成测试使用的歌曲 ID |

## 本地联调流程
1. 启动 MongoDB 与 PostgreSQL，并导入测试数据。
2. 启动服务端：`cargo run -p sigsong-server`。
3. （可选）运行 SDK 集成测试或 iOS/Android 客户端进行联调。

## 测试
- 后端：`cargo test -p sigsong-server`（覆盖歌曲、推荐、搜索、登录等回环流程）。
- SDK：
  - 无网络单元测试：`cargo test -p sigsong-sdk --lib`
  - 端到端测试：
    ```bash
    SIGSONG_SERVER_BASE_URL=http://127.0.0.1:7878/api \
      cargo test -p sigsong-sdk --test full_stack
    ```
    需要服务端、Mongo、Postgres 均已启动。

## 生成 Swift 包
在 `sigsong-sdk` 目录执行：
```bash
IPHONEOS_DEPLOYMENT_TARGET=18.0 \
  cargo swift package --name SigsongSDK --lib-type dynamic --platforms ios
```
生成的 Swift 包位于 `sigsong-sdk/SigsongSDK`，需要同步到 `sigsong-ios/SigSongSDK` 供 Xcode 使用。

> **提示**：动态库会触发 `cargo swift` 的警告（App Store 可能限制），如需上架可视情况切换为静态库。

## iOS 构建
```bash
cd sigsong-ios
xcodebuild \
  -project SigSong.xcodeproj \
  -scheme SigSong \
  -configuration Debug \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  build
```
首次运行前请确保同步了最新的 `SigsongSDK` Swift 包。

## Android 构建
```bash
cd sigsong-android
JAVA_HOME=/path/to/jdk ./gradlew assembleDebug
```
首次执行需要 Gradle 下载 Android 构建插件（若处于离线网络，可将 `gradle-wrapper.properties` 的 `distributionUrl` 指向本地缓存）。构建产物位于 `app/build/outputs/apk/debug/`。

## Demo 登录
当前 Demo 账号：
- 用户名：`demo@sigsong`
- 密码：`sigsong123`

SDK 会在登录成功后缓存 `ClUserInfo` 与 JWT，iOS 客户端的 `UserStore` 会自动清除登录页面并保存会话状态。
