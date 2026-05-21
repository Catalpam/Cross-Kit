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
Cross-Kit 的公开平台 API 保持 state-driven：端上调用同步 action，观察
generated bridge 暴露的 `state` / `items`，不直接订阅 Rust observer 或处理
subscription id。

底层仍然由 Rust VM 定义 observer trait，生成代码负责实现和绑定：
- Rust 侧定义 `VmObserver` trait（带 `on_state` / `on_event`）
- Swift/Kotlin generated bridge 实现该接口并绑定到 UI state
- Rust VM 内部维护当前 state 和 observer 集合，订阅时立即 replay 当前 state

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
- Cross-Kit 不把 Rust async 方法映射成 Swift `async throws` 或 Kotlin `suspend`。
- 登录、刷新、搜索、上传等长任务对端上仍表现为同步 action + observed state：
  `loading`、`progress`、`result`、`notice`、`can_retry` 等都放进业务 state。
- Rust 业务代码可以自行使用线程、runtime 或队列推进任务，但这些细节不进入 generated
  platform API。
- Swift/Kotlin generated bridge 负责把 observer 回调切回 SwiftUI/Compose 适合的主线程模型。

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

## 5.1 当前 Crate 列表（现状梳理）

- `crates/ck-vm-macros`
  - Cross-Kit 级能力：`#[ck_vm_bridge]` 生成 VM 元数据与 Swift Bridge 源码。
- `example/shared`
  - 示例 Rust 共享库（UniFFI + VM 实现）。
  - 仅依赖 `crates/ck-vm-macros` 来生成桥接元数据。
- `tools/ck-swift-packager`
  - SwiftPM / CocoaPods 打包工具，读取 `ck_vm_metadata` 输出。

说明：宏能力属于 Cross-Kit 本体（`crates/ck-vm-macros`），示例工程只作为“使用方”。

---

## 5.2 AppViewModel 与嵌套 VM 架构（建议）

目标：iOS/Android 只做 UI，所有业务状态与跨 VM 逻辑在 Rust 闭环。

核心思路（参考前端状态管理）：
- 单一状态树（Store）：类似 Redux/Elm，将全局 `AppState` 作为唯一事实来源。
- 单向数据流：UI 读取 State（或 Selector 视图），事件通过 Action/Intent 上行给 Store。
- 子 VM 只是“视图切片”：通过 `AppViewModel` 创建，内部仅持有 `Store` 的 `Arc` 与 selector。
- 跨 VM 协作通过 Store/Effect，不需要层层传递：
  - 共享 Domain Service（例如 UserSession、Navigation、Sync）作为 Effect 处理器
  - 或“事件总线”式的 Action（Store 内部统一处理）

推荐结构（Rust）：
- `AppViewModel`
  - `state: AppState`
  - `dispatch(action: AppAction)`
  - `subscribe(observer)`
  - `make_counter_vm()` / `make_list_vm()` / `make_xxx_vm()`
- `Store`
  - `state: AppState`
  - `reduce(action) -> (new_state, effects)`
  - `run_effects(effects)`
  - `subscribe(selector, observer)` 只推送切片变化（必要时 diff）
- `ChildViewModel`
  - 仅暴露 UI 需要的 `get_state()` / `subscribe()`
  - 内部通过 `Store` 读取/派发

State/Action 方向：
- State：自上而下（Store -> Child VM -> UI）
- Event：自下而上（UI -> Child VM -> AppViewModel/Store）
- 跨 VM：通过 Store 统一调度/Effect 处理，避免子 VM 互持

适配 UniFFI / Swift / Compose：
- AppViewModel 生成子 VM 的工厂方法（桥接层对外公开）
- 子 VM Bridge 只负责订阅与事件派发，不直接修改 UI
- 列表类状态继续使用 diff 推送，减少 FFI 负载

这样在 Rust 侧可以完成跨 VM 逻辑闭环（例如：List 更新后触发 Counter 改变等），UI 仅被动渲染。

---

## 6. 关键落地清单（短期）
- [ ] 把 `tools/ck-swift-packager` 接入 CLI（作为 `cross-kit ios package`）
- [ ] 补齐 Android AAR 自动生成脚本
- [ ] 增加 Rust VM 模板（Login/Search）
- [ ] `init` 生成 demo（SwiftUI + Compose 一致 UI）

---

## 7. 当前已落地（example/shared）

### 7.1 AppViewModel + Store（Rust）
路径：`example/shared/src/lib.rs`

已实现：
- `AppState`：`counter / list_len / last_item / route`
- `Route`：`ListDetail` / `Summary`
- `Store`（单一状态源）：统一管理 App/Counter/List 观察者、路由、ActionLog
- `AppViewModel`：
  - `new(initial: i32)`
  - `subscribe/unsubscribe`
  - `clear_route` / `request_summary`
  - `make_counter_vm` / `make_list_vm`
  - `action_log`
- `CounterViewModel`（由 AppViewModel 工厂创建）：
  - `increment()`：每 3 次触发新增 ListItem + 路由到 `ListDetail`
- `ListViewModel`（diff 列表）：
  - Insert/Update/Remove/Move + 批量 diffs + 排序
  - 列表 index 统一 `i64`
  - 列表项包含 `timestamp_ms` + 中文日期 `date_cn`
  - 订阅时推送 `Insert` diffs（不推全量）
  - 列表从 <2 增长到 >=2 时自动触发 `Summary` 路由（若当前无路由）

编译 & 测试 & 覆盖率：
```
cargo test --manifest-path example/shared/Cargo.toml
cargo llvm-cov --manifest-path example/shared/Cargo.toml --summary-only
```
覆盖率（TOTAL Lines）：97.90%（>=97%）

### 7.2 Swift Package 打包建议命令
```
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./example/shared \
  --package-name CrossKitShared \
  --lib-name cross_kit_shared \
  --targets ios,ios-sim,ios-sim-x86_64 \
  --lib-type static \
  --format spm \
  --swift-bridges
```

输出：
```
example/shared/dist/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/*.swift
  cross_kit_sharedFFI.xcframework
```

### 7.3 Swift 打包工具状态
- 修复 `tools/ck-swift-packager` 里 `TargetKind` 判断与路径类型问题
- 当前用于生成 SwiftPM / CocoaPods 的基本骨架
- 额外支持：同平台多架构会通过 `lipo` 合并（例如 iOS Simulator arm64 + x86_64）
- `--swift-bridges` 现在读取 `ck_vm_bridge` 宏生成的元数据（`ck_vm_metadata`），直接写出宏内生成的 Swift 源码（`swift_code`），不再在打包器侧硬编码模板
- 新增 `crates/ck-vm-macros`：`#[ck_vm_bridge]` 自动收集公开方法签名并生成 Swift Bridge 源码
- 新增宏能力：
  - 支持 AppViewModel 工厂创建子 VM（`factory_type` / `factory_method` / `factory_bridge`）
  - `subscribe` 若返回 `observerId`，Swift Bridge 会在 `deinit` 自动 `unsubscribe`
  - Observer 通过 `ObserverProxy` 弱引用转发到 `@MainActor`
  - `Arc<T>` 自动映射为 Swift 的 `TProtocol`

### 7.3.1 Bridge 生成约定（ck_vm_bridge contract）
> 目的：规范 VM 的最小形态，让宏和打包器稳定生成 Swift Bridge。

必须项（编译期）：
- `impl` 必须标注 `#[ck_vm_bridge(...)]`，且 `mode` 为 `state` 或 `diff_list`
- `#[uniffi::export] impl` 内所有需要被桥接的方法必须是 `pub`

`state` 模式约定：
- 必须有 `get_state() -> StateType`
- 必须有 `subscribe(observer: Arc<dyn Observer>)`
- 若有构造参数，需提供 `#[uniffi::constructor] pub fn new(...) -> Arc<Self>`
- 若子 VM 由 AppViewModel 创建，需在宏参数中声明：
  - `factory_type = "AppViewModel"`
  - `factory_method = "make_xxx_vm"`
  - `factory_bridge = "AppViewModelBridge"`
- Swift Bridge 默认生成：
  - `@Published state: StateType`
  - `init(...)` 调 `getState()` + `subscribe(...)`
  - `observer_method(state: StateType)` 回写 `state`

`diff_list` 模式约定：
- 必须有 `subscribe(observer: Arc<dyn Observer>)`
- 可选 `len/append/insert/update/remove/move/sort/apply_diffs` 等方法
- Swift Bridge 默认生成：
  - `@Published items: [ListItem]`
  - `observer_method(diffs: [ListDiff])` 内置 diff 应用逻辑

命名映射规则：
- Rust `snake_case` 自动转为 Swift `lowerCamelCase`
- Rust `Option<T>` -> Swift `T?`
- Rust `Vec<T>` -> Swift `[T]`
- Rust `unit` -> Swift `Void`

注意事项：
- `subscribe`/`new` 不会生成 Swift 方法（避免暴露内部初始化流程）
- `observer_method` 名称同样遵循 `snake_case` -> `lowerCamelCase`
- 如果 `subscribe` 返回 `i64` 且存在 `unsubscribe(id: i64)`，Swift Bridge 会生成 `observerId` 并在 `deinit` 自动取消订阅

### 7.4 已生成的 Swift Package（example/shared）
已执行：
```
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./example/shared \
  --package-name CrossKitShared \
  --lib-name cross_kit_shared \
  --targets ios,ios-sim,ios-sim-x86_64 \
  --lib-type static \
  --format spm \
  --swift-bridges
```

产物：
```
example/shared/dist/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/cross_kit_shared.swift
  Sources/CrossKitShared/Bridges/AppViewModelBridge.swift
  Sources/CrossKitShared/Bridges/CounterViewModelBridge.swift
  Sources/CrossKitShared/Bridges/ListViewModelBridge.swift
  cross_kit_sharedFFI.xcframework
```
说明：`Bridges/*.swift` 由 `ck_vm_bridge` 宏生成（文件头有 `Generated by ck-vm-macros`）。

### 7.5 iOS 示例接入（SwiftPM）
已在 `example/ios/crosskit-example-ios.xcodeproj` 中添加本地 Swift Package：
- 依赖路径：`../shared/dist/CrossKitShared`
- 目标：`crosskit-example-ios`
- Frameworks 已挂载 `CrossKitShared`
- 已补齐 `packageProductDependencies` 与 `XCSwiftPackageProductDependency`，确保 Xcode 的 Package Dependencies 可见
- SwiftUI 不再需要本地 Bridge 文件，直接使用包内模板

SwiftUI 示例：
- 由 Swift Package 生成 `AppViewModelBridge` / `CounterViewModelBridge` / `ListViewModelBridge`
- 更新 `example/ios/crosskit-example-ios/ContentView.swift`：
  - Counter +1（Rust 触发路由 `ListDetail`）
  - List 增删改移动/排序 demo（diff 推送）
  - Summary 路由自动触发（列表从 <2 增长到 >=2 时由 Rust 决策）
  - 按钮带 `accessibilityIdentifier` 便于 UI Test 点击

### 7.6 iOS 编译验证
```
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```
结果：BUILD SUCCEEDED

### 7.7 List VM（Rust，diff 推送）
新增类型（`example/shared/src/lib.rs`）：
- `ListItem { id: i64, timestamp_ms: i64, date_cn: String }`
- `ListDiff::Insert/Update/Remove/Move`（diff 推送）
- `ListObserver`（`on_diffs(Vec<ListDiff>)`）
- `ListViewModel`（增删改查、移动、排序、批量 diff）

行为要点：
- 初次订阅不会推送全量列表，改为批量 `Insert` diffs
- 支持 `move` 与排序产生的 move diffs
- 提供 `apply_diffs` 用于批量 diff 应用

Rust 单测与覆盖率：
```
cargo test --manifest-path example/shared/Cargo.toml
cargo llvm-cov --all-features --summary-only
```
覆盖率：TOTAL Lines 97.92%（>=97%）

### 7.8 iOS 单测与覆盖率
执行：
```
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'id=A63AE66E-A558-40C5-A937-61AA7712C5E7' \
  -derivedDataPath /tmp/crosskit-example-ios-derived \
  test
```

覆盖率查看：
```
xcrun xccov view --report \
  /tmp/crosskit-example-ios-derived/Logs/Test/Test-crosskit-example-ios-2026.02.06_00-05-51-+0800.xcresult
```
结果：`crosskit-example-ios.app` 100%（>=97%）

---

## 8. 会话记录（对话执行日志）

> 目的：当上下文丢失时，快速恢复「用户要求 + 已完成事项 + 关键坑」。

### 8.1 用户关键要求（原话要点）
- 先实现 Rust VM，再做 iOS。
- Rust 必须 `cargo check` 0 errors 后才能继续下一步。
- iOS 示例工程已由用户用 Xcode 创建：`example/ios`。
- SwiftPM 依赖管理，必须用自写 CLI 生成 Swift bindings + Swift Package。
- 所有改动必须记录到本 MD（本文档）。
- Swift Bridge 模板不要写死，要通过 Rust 侧宏生成（参考 uniffi 思路）。
- 宏生成能力属于 Cross-Kit 本体，example 只是依赖方。
- 需要 AppViewModel + 子 VM 架构，跨 VM 逻辑在 Rust 闭环，UI 纯渲染。
- 列表 VM 必须是 diff 推送（不能全量），需要 Move / 批量 diff / 排序变更。
- 列表 index 统一 `i64`，列表项为时间戳 + 中文日期。
- ObservableList 实例只能被单个 VM 持有（不共享给多个 VM）。
- 覆盖率要求：Rust / iOS 都需 >=97%。
- 路由由 Rust 决策，iOS 仅接受统一路由回调并执行跳转。

### 8.2 已完成事项
- 创建 `example/shared` 并实现 Rust Counter VM（`CounterState`/`CounterObserver`/`CounterViewModel`）。
- `cargo check --manifest-path example/shared/Cargo.toml` 通过（0 errors）。
- 修复 `tools/ck-swift-packager`：
  - `TargetKind` 判断、路径类型转换。
  - 同平台多架构通过 `lipo` 合并（iOS Simulator arm64 + x86_64）。
  - 默认 `xcframework_name = <lib_name>FFI`，避免 SwiftPM 目标名重复。
- 增加 `--swift-bridges`：读取 `ck_vm_bridge` 产出的 `swift_code`，写出 Swift Bridge 源码（不再在打包器硬编码模板）。
- 新增 `crates/ck-vm-macros`：`#[ck_vm_bridge]` 自动收集公开方法签名并生成 Swift Bridge 代码（写入 metadata）。
- 将宏 crate 从 `example/ck-vm-macros` 迁移到 `crates/ck-vm-macros`，示例库通过 path 依赖。
- `ck_vm_metadata` 新增 `swift_code` 校验测试。
- 用 CLI 生成 Swift Package：
  - `--targets ios,ios-sim,ios-sim-x86_64 --swift-bridges`
  - 产物 `example/shared/dist/CrossKitShared`（含 `cross_kit_sharedFFI.xcframework` 与 Bridges）。
- 迁移宏 crate 后重新生成 Swift Package（确保依赖路径正确）。
- 落地 AppViewModel + Store + Route（`Summary`/`ListDetail`）：
  - Counter/List VM 由 AppViewModel 工厂创建
  - Rust 侧 `request_summary` 决策路由
- iOS 示例接入：
  - 本地 Bridge 文件移除（`example/ios/crosskit-example-ios/CounterViewModel.swift`），改用包内模板代码。
  - 更新 `example/ios/crosskit-example-ios/ContentView.swift` 为 Counter + List UI（读 `state`，按钮使用闭包调用）。
  - 手工编辑 `example/ios/crosskit-example-ios.xcodeproj/project.pbxproj`，添加本地 Swift Package 引用和 Frameworks。
  - 测试目标补齐 `CrossKitShared` 包依赖。
- iOS Demo 增加 List UI（增删改移动/排序），并添加按钮 `accessibilityIdentifier`。
- Summary 路由改为 Rust 自动触发（列表从 <2 增长到 >=2 时）。
- UI Tests 更新为在第二次插入后等待 Summary 并返回，再继续后续操作。
- iOS 单测改为 async 等待状态/diff 回调（避免回调异步导致断言失败）。
- 新增 List VM（diff 推送 + Move/Batch/排序 + index i64 + timestamp/date）。
- 更新 List Bridge 测试方法名（`insertWithTimestamp`/`updateWithTimestamp`/`moveItem`/`removeAt`）。
- Swift Bridge 生成更新：ObserverProxy + `deinit` 自动取消订阅 + 工厂初始化参数。
- 重新生成 Swift Package（`example/shared/dist/CrossKitShared`）以包含最新 Rust 逻辑。
- Rust 覆盖率更新：TOTAL Lines 97.92%（>=97%）。
- iOS 单测与覆盖率通过（`crosskit-example-ios.app` 100%）。

### 8.3 运行与验证
- `cargo llvm-cov --manifest-path example/shared/Cargo.toml --summary-only` 通过（TOTAL Lines 97.92%）。
- `xcodebuild test` 通过（iOS Simulator `A63AE66E-A558-40C5-A937-61AA7712C5E7`）。
- `xcrun xccov view --report /tmp/crosskit-example-ios-derived/Logs/Test/Test-crosskit-example-ios-2026.02.06_00-05-51-+0800.xcresult`：`crosskit-example-ios.app` 100%
- 解决过的问题：
  - Swift `Combine` 未导入导致 `ObservableObject`/`@Published` 报错。
  - XCFramework 名称与 SwiftPM target 名冲突导致 `duplicate target`。
  - Simulator 缺 x86_64 架构导致链接失败。
  - `Button(action: vm.increment)` 返回值不匹配，改为闭包调用。
  - List Bridge 方法名与测试不一致，按桥接方法名更新测试。
  - `xcodebuild test` 找不到 `iPhone 16`，改用 `iPhone 17 Pro Max` 设备 id。
  - Swift Bridge 初始化出现 “self used before initialized”，通过 `observer` 改为可选并延后赋值修复。
  - 异步回调导致单测断言过早，改为 async 等待条件。
- 当前注意点：
  - Xcode UI 若不显示 Package Dependencies，需确保 `packageProductDependencies` 与 `XCSwiftPackageProductDependency` 正确写入 pbxproj，并重启 Xcode / Reset Package Caches。

### 8.4 关键路径索引
- Rust VM：`example/shared/src/lib.rs`
- Rust VM 宏：`crates/ck-vm-macros/src/lib.rs`
- VM 元数据输出：`example/shared/src/bin/ck_vm_metadata.rs`
- Swift Package 输出：`example/shared/dist/CrossKitShared`
- iOS 工程：`example/ios/crosskit-example-ios.xcodeproj`
- SwiftUI 入口：`example/ios/crosskit-example-ios/ContentView.swift`
- Swift Bridge：`example/shared/dist/CrossKitShared/Sources/CrossKitShared/Bridges/*.swift`
- iOS UI Tests：`example/ios/crosskit-example-iosUITests/crosskit_example_iosUITests.swift`
