# Cross-Kit CLI 规划与 Rust VM 技术方案

本文档面向「从零上手」的 AI/开发者，目标是明确 Cross-Kit CLI 的里程碑、Rust 下沉 ViewModel 的实现方式，以及 iOS/Android 的打包与集成流程（先文档，后落地）。

## 1. 里程碑（Milestones）

### M0：CLI 骨架与工作流对齐
- 子命令规划：
  - `cross-kit ios package`：编译 Rust + UniFFI，产出 Swift Package / CocoaPods
  - `cross-kit android package`：编译 Rust，产出 AAR（含 .so + Kotlin bindings）
  - `cross-kit init`：生成可运行 demo（三端：Rust shared + iOS + Android）
  - `cross-kit gen vm`：从规范生成 Rust VM + SwiftUI + Compose 的映射代码
- 统一配置入口：`cross-kit.toml`（或 `cross-kit.yaml`）
- 清晰的产物目录结构（比如 `dist/ios`, `dist/android`）

### M1：iOS 打包链路可用
- Rust → Swift bindings → XCFramework → Swift Package
- 支持：目标平台选择（iOS / iOS 模拟器 / macOS）
- 支持：静态 / 动态库（动态库需提示 App Store 风险）
- 产物可直接被 Xcode 以 SwiftPM 或 CocoaPods 引用

### M2：Android 打包链路可用
- Rust → .so（arm64-v8a / x86_64 等 ABI）
- UniFFI Kotlin bindings 自动生成
- 产出 AAR（可本地 Maven 或直接模块引用）

### M3：`init` 产出三端 demo
- Rust shared 包含最小 VM（如 Login / Search）
- iOS：SwiftUI + 绑定 Rust VM
- Android：Compose + 绑定 Rust VM
- 目标：开箱即跑，展示 “一套 VM 三端复用” 的闭环

### M4：AI Agent 代码生成
- 规范驱动（Schema/DSL）描述页面与状态
- 输出：Rust VM、SwiftUI、Compose 的一致 UI 结构
- 保障 SwiftUI / Compose 组件一一映射

### M5：工程化与发布
- CI 打包（macOS 生成 iOS 包，Linux/Windows 生成 Android 包）
- 产物版本化与缓存策略
- 文档与模板完善

---

## 2. Rust 下沉 ViewModel 的技术方案（基于现有示例演化）

### 2.1 参考现有工程的关键模式
从 `old-example/sigsong-sdk` 与 `old-example/sigsong-ios` / `old-example/sigsong-android` 可观察到：
- Rust 侧通过 `InvokeManager` 统一暴露 API（UniFFI `#[uniffi::export]`）
- Rust 侧定义 `InvokeFFI` trait（`#[uniffi::export(with_foreign)]`），由 Swift/Kotlin 实现
- iOS/Android 通过 `InvokeFFI` 回调接收 Toast / 路径等平台能力
- Swift/Kotlin 侧持有 `InvokeManager` 实例，作为业务入口

这个模式本质上就是 **“Rust 侧业务中心 + 平台能力回调”**。接下来我们把 ViewModel 状态也下沉到 Rust。

对照文件（便于零基础 AI 快速定位）：
- Rust 入口：`old-example/sigsong-sdk/src/client/invoke.rs`
- Rust 回调接口：`old-example/sigsong-sdk/src/client/client_ability.rs`
- Rust API 暴露：`old-example/sigsong-sdk/src/native/api/*.rs`
- iOS 回调桥接：`old-example/sigsong-ios/InvokeKit/Sources/InvokeKit/Invoke.swift`
- iOS SDK 初始化：`old-example/sigsong-ios/InvokeKit/Sources/InvokeKit/InvokeKit.swift`
- Android 回调桥接：`old-example/sigsong-android/app/src/main/java/com/sigsong/android/SigSongApp.kt`

### 2.2 核心设计：Rust VM = 状态机 + 事件 + 意图
建议将 ViewModel 抽象为：
- `State`：纯数据快照（UniFFI `Record`），可序列化/可比较
- `Event/Effect`：一次性事件（Toast、导航、弹窗）
- `Intent`：来自 UI 的操作（按钮点击、输入改变、刷新）

数据流（单向）：
```
UI Intent -> Rust VM -> State 更新 -> 回调 -> SwiftUI/Compose 刷新
```

### 2.3 Rust 侧结构建议
```
shared/
  src/
    vm/
      home_vm.rs
      login_vm.rs
      search_vm.rs
    state/
      home_state.rs
      login_state.rs
    intent/
      home_intent.rs
```

Rust VM 基本模板（思路）：
1. `#[derive(uniffi::Object)]` 的 VM 结构体
2. `#[derive(uniffi::Record)]` 的 State
3. `#[derive(uniffi::Enum)]` 的 Event
4. `#[uniffi::export(with_foreign)]` 的观察者接口（Observer）
5. VM 提供 `subscribe(observer)` 与 `dispatch(intent)`

### 2.4 观察者（Observer）桥接
与 `InvokeFFI` 类似：
- Rust 侧定义 `VmObserver` trait（带 `on_state` / `on_event`）
- Swift/Kotlin 实现该接口并绑定到 UI
- Rust VM 内部通过 `tokio::sync::watch` 或 `broadcast` 管理订阅

Rust 伪代码结构：
```rust
#[derive(uniffi::Record)]
pub struct LoginState {
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub current_user: Option<ClUserInfo>,
}

#[derive(uniffi::Enum)]
pub enum LoginEvent {
    ShowToast { message: String },
    NavigateHome,
}

#[uniffi::export(with_foreign)]
pub trait LoginObserver: Send + Sync {
    fn on_state(&self, state: LoginState);
    fn on_event(&self, event: LoginEvent);
}

#[derive(uniffi::Object)]
pub struct LoginViewModel { /* state + runtime */ }

#[uniffi::export]
impl LoginViewModel {
    pub fn subscribe(&self, observer: Arc<dyn LoginObserver>) { /* ... */ }
    pub fn dispatch(&self, intent: LoginIntent) { /* ... */ }
}
```

### 2.5 SwiftUI 与 Compose 的一一映射策略
保持 UI 结构一致：
- SwiftUI：`ObservableObject` + `@Published var state: LoginState`
- Compose：`remember { mutableStateOf(state) }` 或 `StateFlow`
- UI 仅依赖 `state` 渲染，不直接调用 Rust API

SwiftUI 示例（概念）：
```swift
@MainActor
final class LoginVM: ObservableObject, LoginObserver {
    @Published var state: LoginState = .init(...)
    func onState(_ state: LoginState) { self.state = state }
    func onEvent(_ event: LoginEvent) { /* Toast / Navigation */ }
}
```

Compose 示例（概念）：
```kotlin
class LoginVM : LoginObserver {
    var state by mutableStateOf(LoginState(...))
        private set
    override fun onState(state: LoginState) { this.state = state }
    override fun onEvent(event: LoginEvent) { /* Snackbar / Nav */ }
}
```

### 2.6 并发与线程模型建议
- Rust 内部使用 `tokio` runtime
- Rust VM 中所有 IO / 网络 / DB 任务走 `async`
- Swift/Kotlin 回调回主线程（SwiftUI/Compose 的 UI 线程）

### 2.7 建议的 VM 生命周期
- UI 启动时创建 VM 并订阅
- 页面销毁时释放 VM（或调用 `close()`）
- Rust 侧维护 `Arc` + `Weak`，避免循环引用

---

## 3. iOS 打包方案（文档版 + 简化实现）

### 3.1 目标流程（与 `cargo-swift` 一致但更轻量）
1. `cargo build --target <target>` 编译 Rust
2. `uniffi_bindgen` 生成 Swift bindings（.swift + .h + .modulemap）
3. `xcodebuild -create-xcframework` 生成 XCFramework
4. 生成 SwiftPM / CocoaPods 包结构

### 3.2 已添加的简化脚本（起步版）
已在本仓库新增一个简化工具：
```
tools/ck-swift-packager
```
它实现了：
- UniFFI Swift bindings 生成
- XCFramework 打包
- SwiftPM / CocoaPods 目录结构输出

示例用法（静态库，iOS + 模拟器）：
```bash
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./old-example/sigsong-sdk \
  --package-name SigsongSDK \
  --lib-name sig_song_sdk \
  --targets ios,ios-sim \
  --lib-type static \
  --format spm
```

输出位置（默认）：
```
<crate>/dist/<PackageName>/
  Package.swift
  Sources/<PackageName>/*.swift
  <PackageName>.xcframework
```

> 说明：动态库模式仍需在 App Store 场景下谨慎评估。

---

## 4. Android 打包方案（文档版）

### 4.1 选择的主流方式：AAR + 本地 Maven
Android 主流依赖方式：
- **AAR**（含 .so + Kotlin 绑定）
- 分发方式：
  - 本地 Maven（`mvnLocal()` 或 `maven { url(...) }`）
  - 直接工程模块依赖

### 4.2 推荐打包流程
1. 使用 `cargo ndk` 生成各 ABI 的 `.so`
2. 用 `uniffi-bindgen` 生成 Kotlin 绑定
3. 将 `.so` 放入 `jniLibs/<abi>/`
4. Kotlin bindings 放入 `src/main/java/...`
5. Gradle 生成 AAR

### 4.3 与现有示例的对应
在 `old-example/sigsong-android`：
- `.so` 位于 `app/src/main/jniLibs/`
- Kotlin 绑定位于 `app/src/main/java/com/sigsong/sdk/...`

下一步 CLI 应将其变为：
```
cross-kit android package \
  --targets arm64-v8a,x86_64 \
  --out dist/android
```

输出：
```
dist/android/
  cross-kit-sdk.aar
  pom.xml (optional)
```

---

## 5. 未来 CLI 结构（建议）

```
cross-kit init
cross-kit ios package
cross-kit android package
cross-kit gen vm
cross-kit gen ui
```

配置示例（cross-kit.toml）：
```toml
[rust]
crate = "./shared"
lib_name = "crosskit_shared"

[ios]
package_name = "CrossKitShared"
targets = ["ios", "ios-sim"]
lib_type = "static"

[android]
abis = ["arm64-v8a", "x86_64"]
group_id = "com.crosskit"
artifact_id = "shared"
```

---

## 6. 关键落地清单（短期）
- [ ] 把 `tools/ck-swift-packager` 接入 CLI（作为 `cross-kit ios package`）
- [ ] 补齐 Android AAR 自动生成脚本
- [ ] 增加 Rust VM 模板（Login/Search）
- [ ] `init` 生成 demo（SwiftUI + Compose 一致 UI）

---

## 7. 当前已落地（example/shared）

### 7.1 Counter VM（Rust）
路径：`example/shared/src/lib.rs`

已实现：
- `CounterState`（`value: i32`）
- `CounterObserver`（`on_state` 回调）
- `CounterViewModel`：
  - `new(initial: i32)`
  - `subscribe(observer)`
  - `increment() -> CounterState`
  - `get_state() -> CounterState`

用途：作为最小 demo，UI 只绑定 `CounterState` 渲染，点击 +1 触发 `increment()`，Rust 侧广播最新状态。

编译验证：
```
cargo check --manifest-path example/shared/Cargo.toml
```
结果：0 errors（已通过）

### 7.2 Swift Package 打包建议命令
```
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./example/shared \
  --package-name CrossKitShared \
  --lib-name cross_kit_shared \
  --targets ios,ios-sim \
  --lib-type static \
  --format spm
```

输出：
```
example/shared/dist/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/*.swift
  CrossKitShared.xcframework
```

### 7.3 Swift 打包工具状态
- 修复 `tools/ck-swift-packager` 里 `TargetKind` 判断与路径类型问题
- 当前用于生成 SwiftPM / CocoaPods 的基本骨架
- 额外支持：同平台多架构会通过 `lipo` 合并（例如 iOS Simulator arm64 + x86_64）

### 7.4 已生成的 Swift Package（example/shared）
已执行：
```
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./example/shared \
  --package-name CrossKitShared \
  --lib-name cross_kit_shared \
  --targets ios,ios-sim,ios-sim-x86_64 \
  --lib-type static \
  --format spm
```

产物：
```
example/shared/dist/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/cross_kit_shared.swift
  cross_kit_sharedFFI.xcframework
```

### 7.5 iOS 示例接入（SwiftPM）
已在 `example/ios/crosskit-example-ios.xcodeproj` 中添加本地 Swift Package：
- 依赖路径：`../shared/dist/CrossKitShared`
- 目标：`crosskit-example-ios`
- Frameworks 已挂载 `CrossKitShared`
- 已补齐 `packageProductDependencies` 与 `XCSwiftPackageProductDependency`，确保 Xcode 的 Package Dependencies 可见

SwiftUI 示例：
- 新增 `example/ios/crosskit-example-ios/CounterViewModel.swift`（`CounterViewModelBridge`）
- 更新 `example/ios/crosskit-example-ios/ContentView.swift` 展示 Counter +1

### 7.6 iOS 编译验证
```
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```
结果：BUILD SUCCEEDED

---

## 8. 会话记录（对话执行日志）

> 目的：当上下文丢失时，快速恢复「用户要求 + 已完成事项 + 关键坑」。

### 8.1 用户关键要求（原话要点）
- 先实现 Rust VM，再做 iOS。
- Rust 必须 `cargo check` 0 errors 后才能继续下一步。
- iOS 示例工程已由用户用 Xcode 创建：`example/ios`。
- SwiftPM 依赖管理，必须用自写 CLI 生成 Swift bindings + Swift Package。
- 所有改动必须记录到本 MD（本文档）。

### 8.2 已完成事项
- 创建 `example/shared` 并实现 Rust Counter VM（`CounterState`/`CounterObserver`/`CounterViewModel`）。
- `cargo check --manifest-path example/shared/Cargo.toml` 通过（0 errors）。
- 修复 `tools/ck-swift-packager`：
  - `TargetKind` 判断、路径类型转换。
  - 同平台多架构通过 `lipo` 合并（iOS Simulator arm64 + x86_64）。
  - 默认 `xcframework_name = <lib_name>FFI`，避免 SwiftPM 目标名重复。
- 用 CLI 生成 Swift Package：
  - `--targets ios,ios-sim,ios-sim-x86_64`
  - 产物 `example/shared/dist/CrossKitShared`（含 `cross_kit_sharedFFI.xcframework`）。
- iOS 示例接入：
  - 新增 `example/ios/crosskit-example-ios/CounterViewModel.swift`（`CounterViewModelBridge`）。
  - 更新 `example/ios/crosskit-example-ios/ContentView.swift` 为 Counter UI。
  - 手工编辑 `example/ios/crosskit-example-ios.xcodeproj/project.pbxproj`，添加本地 Swift Package 引用和 Frameworks。

### 8.3 运行与验证
- `xcodebuild -list` 与 `xcodebuild build` 可成功（命令行）。
- 解决过的问题：
  - Swift `Combine` 未导入导致 `ObservableObject`/`@Published` 报错。
  - XCFramework 名称与 SwiftPM target 名冲突导致 `duplicate target`。
  - Simulator 缺 x86_64 架构导致链接失败。
- 当前注意点：
  - Xcode UI 若不显示 Package Dependencies，需确保 `packageProductDependencies` 与 `XCSwiftPackageProductDependency` 正确写入 pbxproj，并重启 Xcode / Reset Package Caches。

### 8.4 关键路径索引
- Rust VM：`example/shared/src/lib.rs`
- Swift Package 输出：`example/shared/dist/CrossKitShared`
- iOS 工程：`example/ios/crosskit-example-ios.xcodeproj`
- SwiftUI 入口：`example/ios/crosskit-example-ios/ContentView.swift`
- Swift Bridge：`example/ios/crosskit-example-ios/CounterViewModel.swift`
