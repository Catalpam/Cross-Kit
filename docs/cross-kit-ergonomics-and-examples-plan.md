# Cross-Kit Ergonomics and Examples Plan

本文档用于规划 Step 8 之后的 API 易用性重构和 examples 扩展。目标不是只修当前发现的 `vm_bridge` 参数样板问题，而是把 Cross-Kit 的调用面整体往“业务代码少写模板、端上少感知桥接细节、Cross-Kit 内部承担更多生成和生命周期工作”的方向推进。

## 0. 当前判断

当前 0-7 步已经把仓库角色拆清楚：

- Rust 用户依赖 `crates/cross-kit`。
- CLI 负责 iOS Swift Package / XCFramework 和 Android AAR / Maven artifact。
- iOS 和 Android example 都依赖生成后的端上库。
- `examples/counter-list` 已经覆盖 state VM、diff list VM、parent factory、route 等关键能力。

但当前调用体验仍偏工具原型：

- Rust VM 宏参数显式但字符串化，重命名类型或方法时容易漂移。
- Rust SDK 业务代码仍手写较多 observer map、subscription id、notify clone 逻辑。
- metadata binary 需要手写 VM 列表和 JSON 拼接。
- iOS 示例需要手动创建 `AppViewModelBridge`、`CounterViewModelBridge`、`ListViewModelBridge` 并维护 parent-child 构造关系。
- Android 示例需要手动 `remember` 多个 bridge，并在 `DisposableEffect` 里逐个 `close()`。
- Android config 同时暴露 Step 6 generated-source 调试字段和 Step 7 AAR packaging 字段，普通用户会困惑。
- 当前只有 `counter-list` 一个 example，不足以证明 Cross-Kit 适合哪些业务形态。

## 0.1 本轮 Review 采纳情况

外部 review 中大部分建议成立，已纳入本文：

- Step 8 拆成 Step 8A / Step 8B 两个独立提交阶段。先做字符串兼容的推断和编译期校验，再做 Rust path 参数语法。
- Step 8A 明确增加 `trybuild` 编译失败测试。
- 宏生成 metadata 后必须在宏展开期 validate，并通过 `compile_error!` 报错，不能等到 package 阶段。
- Step 9 明确处理 `SubscriptionId` 对 Swift/Kotlin type mapping 的影响。
- Step 9 明确 `ObserverSet` 不应放入 `StoreInner`，状态锁和 observer set 锁要分离。
- Step 9 补充订阅立即同步当前 state / 当前 list diffs 的验收。
- Step 11 增加 root graph contract，明确 root/container 配置、child factory 形状、close 顺序和歧义报错。
- Step 11 明确 iOS root container 必须转发 child `objectWillChange`，否则 `@StateObject var kit` 不能可靠刷新 nested bridge。
- Step 12 明确 Android Maven config 要从 core config、CLI mapping、packager options 一路穿透到 Gradle/POM。

没有采纳的部分：

- “从 observer trait 自动推断 callback method”暂不做。Rust proc macro 不能可靠读取同 crate 任意 trait 定义，短期采用 mode 默认值和显式 override。

## 1. 设计原则

后续所有步骤按这些原则判断是否值得做：

- 业务 Rust 代码只描述状态、动作、订阅关系和必要业务逻辑，不手写平台细节。
- 端上代码只面对 generated bridge / generated root container，不感知 Rust FFI、UniFFI、native library、observer proxy、subscription id。
- 默认写法尽可能短；高级用户可以 override，但普通 example 不应该先展示一堆配置。
- 生成代码可以多做事，业务代码不能为了生成器迁就太多模板。
- 每一步必须独立 commit，独立验收，不能一口气重构完。

## 2. 当前三端可优化点

### 2.1 Rust SDK 侧

当前写法：

```rust
#[vm_bridge(
    swift_bridge = "CounterViewModelBridge",
    mode = "state",
    observer = "CounterObserver",
    observer_method = "on_state",
    state_type = "CounterState",
    factory_type = "AppViewModel",
    factory_method = "make_counter_vm",
    factory_bridge = "AppViewModelBridge"
)]
#[uniffi::export]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
    pub fn increment(&self) -> CounterState;
    pub fn get_state(&self) -> CounterState;
}
```

问题：

- `swift_bridge` / `state_type` / `observer` / `factory_type` 等本质上能从 `impl` 方法签名推断，但现在都要手写。
- 参数是字符串，IDE refactor 不可靠。
- `swift_bridge` 是历史命名，公开用法应该统一为 `bridge`。
- `Store` 里 observer 管理样板很重，业务代码要维护 `HashMap<i64, Arc<dyn Observer>>`、递增 id、unsubscribe、notify。
- `ck_vm_metadata.rs` 需要手写 VM 列表和 JSON array 拼接。

目标写法：

```rust
#[cross_kit::vm_bridge(mode = "state")]
#[uniffi::export]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> cross_kit::SubscriptionId;
    pub fn unsubscribe(&self, id: cross_kit::SubscriptionId);
    pub fn increment(&self) -> CounterState;
    pub fn get_state(&self) -> CounterState;
}
```

必要时才 override：

```rust
#[cross_kit::vm_bridge(
    mode = "state",
    bridge = "CounterBridge",
    factory = AppViewModel::make_counter_vm
)]
```

### 2.2 iOS 侧

当前写法：

```swift
@StateObject private var appVm: AppViewModelBridge
@StateObject private var counterVm: CounterViewModelBridge
@StateObject private var listVm: ListViewModelBridge

init() {
    let app = AppViewModelBridge(initial: 0)
    _appVm = StateObject(wrappedValue: app)
    _counterVm = StateObject(wrappedValue: CounterViewModelBridge(app: app))
    _listVm = StateObject(wrappedValue: ListViewModelBridge(app: app))
}
```

问题：

- 端上知道 parent bridge 和 child bridge 的构造关系。
- 多个 `@StateObject` 初始化样板偏重。
- 业务 View 负责把 root 和 child bridge 组合起来。

目标写法：

```swift
@StateObject private var kit = CrossKitSharedBridge(initial: 0)

var body: some View {
    Text("\(kit.counter.state.value)")
    Button("+1") { kit.counter.increment() }
    List(kit.list.items, id: \.id) { item in ... }
}
```

其中 `CrossKitSharedBridge` 是生成的 root container，内部持有 root VM 和 child VMs，并负责生命周期。

### 2.3 Android 侧

当前写法：

```kotlin
val appVm = remember { AppViewModelBridge(initial = 0) }
val counterVm = remember(appVm) { appVm.makeCounterVm() }
val listVm = remember(appVm) { appVm.makeListVm() }

DisposableEffect(Unit) {
    onDispose {
        listVm.close()
        counterVm.close()
        appVm.close()
    }
}
```

问题：

- Compose 业务代码知道 child bridge 创建关系。
- 业务代码必须记得按顺序 close。
- 如果以后 VM 数量变多，入口样板会线性膨胀。

目标写法：

```kotlin
val kit = rememberCrossKitShared(initial = 0)

Text("Counter: ${kit.counter.state.value}")
Button(onClick = { kit.counter.increment() }) { Text("+1") }
LazyColumn {
    items(kit.list.items, key = { it.id }) { item -> ... }
}
```

`rememberCrossKitShared` 由 Android 端上库生成，内部创建 bridges 并在 `DisposableEffect` 中统一 close。

### 2.4 CLI / Config 侧

当前 `cross-kit.toml` 的 `[android]` 同时包含：

- `output` / `jni_libs_output`：Step 6 generated-source 调试路径。
- `package_output` / `gradle_project_output` / `module_name`：Step 7 AAR packaging 路径。

问题：

- 普通用户只应该看到 AAR/Maven packaging 配置。
- generated-source flow 是调试工具，不应该成为 example 默认认知。
- Android artifact 的 group/version 目前由 packager 写死，真实工程需要配置。

目标：

- 主配置只保留端上库 packaging 所需字段。
- 调试命令可以继续存在，但默认路径由 CLI 推断，或者迁到 `[android.codegen]`。
- Android Maven coordinates 明确可配置：`group_id`、`artifact_id`、`version`。

## 3. 执行规则

后续每个 Step 都必须满足：

- 每个 Step 是一个独立 commit。
- Step 开始前确认当前工作区状态，不覆盖用户未提交改动。
- Step 完成后先不提交。
- 必须补充有意义测试，不能只跑历史测试。
- 测试补充必须分两路：
  - 当前执行者自行根据改动面补 case。
  - 另调一个独立 subagent 阅读当前 Step 目标、未提交 diff 和三端相关代码，提出至少 5 条、至多 20 条补充测试 case。
- 提交前必须合并筛选这些 case，并尽量覆盖 Rust、iOS、Android 三端；如果某端无法补自动化 case，必须用 package/build/generated-source 断言作为验收补足。
- Rust workspace 目标行覆盖率仍要求 `> 97%`。
- 跑完该 Step 的验收命令后，调用独立 subagent 阅读本文档和未提交 diff 做 review。
- 根据 subagent review 修复问题后重新跑相关测试。
- 最多 6 轮 review；如果 6 轮仍有无法解决的问题，停下来向用户确认。
- 只有测试、覆盖率、端上验收、review 都通过后才能 commit。
- commit 后才能进入下一 Step。

标准基础验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --exclude cross-kit-packager-ios --summary-only
```

iOS 相关 Step 额外验收：

```bash
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Android 相关 Step 额外验收：

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/counter-list/cross-kit.toml

cd examples/counter-list/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug
```

## 4. Step 8A: 字符串兼容的宏参数推断与编译期校验

目标：让 Rust SDK 用户能用更短、更 Rust-native 的 `vm_bridge` 写法，同时保留旧参数兼容。

### 4.1 具体改动

- 先不引入 Rust path 参数语法，本 Step 只在现有字符串参数能力内做推断和校验。原因是当前 `cross-kit-macros` 的参数解析只接受 `LitStr`，直接支持 `factory = AppViewModel::make_counter_vm` / `diff = ListDiff` 需要替换成混合 AST parser，风险应独立提交。
- `cross-kit-macros` 支持从 `impl` 推断：
  - `bridge`: 默认 `{RustType}Bridge`。
  - `state_type`: `get_state()` 返回类型。
  - `observer`: `subscribe(observer: Arc<dyn XxxObserver>)` 参数类型。
  - `observer_method`: 不承诺读取 observer trait 定义。默认：
    - state 模式默认 `on_state`，可通过 `observer_method = "..."` override。
    - diff_list 模式默认 `on_diffs`，可通过 `observer_method = "..."` override。
  - `diff_type` / `list_item_type`: diff_list 模式本 Step 仍使用字符串字段，缺失时报编译期错误。
  - `factory_type` / `factory_method`: 本 Step 仍使用字符串字段，缺失时报编译期错误。
  - `factory_bridge`: 默认 `{factory_type}Bridge`。
- 公共示例统一使用 `bridge`，不再展示 `swift_bridge`。
- `swift_bridge` 保留为 deprecated compatibility alias，不删除。
- 错误信息要指向缺失的推断来源，例如 “state VM requires get_state() or state_type override”。
- 将 metadata validation 前移到宏展开期：
  - 宏生成 IR 后立即用 `cross-kit-core` 解析和 validate。
  - Swift 兼容字段生成失败时不能再吞成空字符串。
  - 所有失败都用 `syn::Error::new_spanned(...).to_compile_error()` 或等价方式变成编译错误。
  - package 阶段不应该才发现宏 contract 错误。

### 4.2 目标写法

```rust
#[cross_kit::vm_bridge(mode = "state")]
#[uniffi::export]
impl AppViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self>;
    pub fn subscribe(&self, observer: Arc<dyn AppObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
    pub fn get_state(&self) -> AppState;
}

#[cross_kit::vm_bridge(
    mode = "state",
    factory_type = "AppViewModel",
    factory_method = "make_counter_vm"
)]
#[uniffi::export]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
    pub fn get_state(&self) -> CounterState;
}

#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff_type = "ListDiff",
    list_item_type = "ListItem",
    factory_type = "AppViewModel",
    factory_method = "make_list_vm"
)]
#[uniffi::export]
impl ListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn ListObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
}
```

### 4.3 新增测试

- 只写 `mode = "state"` 能推断 bridge/state/observer/observer_method。
- diff_list 能推断 observer，并默认 `observer_method = "on_diffs"`。
- diff_list 缺失 `diff_type` 或 `list_item_type` 时给出清晰编译错误。
- 旧 `swift_bridge` / `state_type` / `factory_type` 字段仍兼容。
- 用户显式 override 优先级高于推断值。
- 重命名方法导致推断失败时错误可读。
- 增加 `trybuild` 编译失败测试，覆盖：
  - state VM 缺失 `get_state`。
  - subscribe 参数不是 `Arc<dyn Observer>`。
  - diff_list 缺失 `diff_type` / `list_item_type`。
  - factory_type/factory_method 不完整。
  - Swift codegen / metadata validate 不再被吞错。

### 4.4 验收

- 基础验收命令通过，覆盖率 > 97%。
- `examples/counter-list/shared` 的 state VM 改用最短可推断写法；diff_list 仍使用字符串 diff/list item 字段。
- metadata snapshot 更新且 review 确认没有改变外部生成 API 名称。
- `trybuild` pass/fail fixtures 纳入测试。
- iOS package + xcodebuild 通过。
- Android package + Gradle assembleDebug 通过。
- subagent review 最多 6 轮。

## 5. Step 8B: Rust path 宏参数语法

目标：在 Step 8A 稳定后，把仍然字符串化的 factory/diff/item 字段改成 Rust path 参数，进一步减少重命名漂移。

### 5.1 具体改动

- 将 `cross-kit-macros` 参数解析从 `HashMap<String, String> + LitStr` 改成混合 AST 参数模型：
  - 字符串字段继续支持：`bridge = "..."`、`observer_method = "..."`。
  - path 字段支持：`factory = AppViewModel::make_counter_vm`、`diff = ListDiff`、`item = ListItem`。
  - legacy 字段继续支持：`factory_type = "..."`、`factory_method = "..."`、`diff_type = "..."`、`list_item_type = "..."`。
- path 字段通过 `syn::Path` 保留 token span，用于更精确的编译错误。
- path 字段序列化到 IR 时仍落成稳定字符串：
  - `factory = AppViewModel::make_counter_vm` -> `factory.rust_type = "AppViewModel"`、`factory.method = "make_counter_vm"`。
  - `diff = ListDiff` -> `diff_type = "ListDiff"`。
  - `item = ListItem` -> `list_item_type = "ListItem"`。
- 冲突规则：
  - 新 path 字段和旧字符串字段同时出现且值不一致时报编译错误。
  - 显式 `bridge = "..."` 优先于默认 `{RustType}Bridge`。

### 5.2 目标写法

```rust
#[cross_kit::vm_bridge(mode = "state", factory = AppViewModel::make_counter_vm)]
#[uniffi::export]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
    pub fn get_state(&self) -> CounterState;
}

#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff = ListDiff,
    item = ListItem,
    factory = AppViewModel::make_list_vm
)]
#[uniffi::export]
impl ListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn ListObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
}
```

### 5.3 新增测试

- path 参数解析成功并生成与旧字符串字段相同的 IR。
- path 与 legacy 字符串字段冲突时报编译错误。
- `factory = AppViewModel`、`factory = AppViewModel::nested::make` 等不支持形状给出清晰错误。
- `diff = some::ListDiff` 的序列化规则明确并有测试。
- 继续保留 Step 8A 的 trybuild 失败测试。

### 5.4 验收

- `examples/counter-list/shared` 全部改用 path 语法。
- metadata snapshot 只因字段输入方式变化而不改变对外生成 API。
- 基础验收 + iOS/Android 端上验收通过。
- subagent review 最多 6 轮。

## 6. Step 9: Rust 订阅样板下沉到 cross-kit runtime

目标：减少 Rust 业务代码对 observer map、subscription id、clone notify 的手写。

### 6.1 具体改动

- 在 `crates/cross-kit` 增加轻量 runtime helper：

```rust
pub type SubscriptionId = i64;

pub struct ObserverSet<O: ?Sized> { ... }

impl<O: ?Sized> ObserverSet<O> {
    pub fn subscribe(&self, observer: Arc<O>) -> SubscriptionId;
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool;
    pub fn notify(&self, f: impl FnMut(&Arc<O>));
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
}
```

- `ObserverSet` 内部管理 id、HashMap 和 clone snapshot。
- `SubscriptionId = i64` 会影响 generator type mapping。本 Step 必须同时处理：
  - 宏归一化 `SubscriptionId` / `cross_kit::SubscriptionId` / `crate::...::SubscriptionId` 为 `i64`；或
  - `cross-kit-codegen` 显式把这些别名映射到 Swift `Int64`、Kotlin `Long`。
  - 两者至少做一个，并补测试防止端上生成 `cross_kit::SubscriptionId` 这种无效类型。
- `examples/counter-list/shared` 将 `app_observers`、`counter_observers`、`list_observers` 替换为 `ObserverSet`，但不要直接塞回 `StoreInner`。
- 推荐结构：

```rust
struct Store {
    inner: Arc<Mutex<StoreInner>>,
    app_observers: ObserverSet<dyn AppObserver>,
    counter_observers: ObserverSet<dyn CounterObserver>,
    list_observers: ObserverSet<dyn ListObserver>,
}
```

- 状态锁和 observer set 锁分离，避免 `StoreInner` 锁内再锁 `ObserverSet` 造成锁嵌套。
- notify 前从状态锁里算出状态/diffs，释放状态锁后再 notify observer。
- 业务 Store 仍保留状态机逻辑，不把业务状态强塞进 Cross-Kit。

### 6.2 目标写法

```rust
let id = self.counter_observers.subscribe(observer);
self.counter_observers.notify(|observer| {
    observer.on_state(counter_state.clone());
});
```

### 6.3 新增测试

- subscribe 返回递增且唯一的 id。
- unsubscribe 后不再收到 notify。
- notify 时允许 observer 在回调里触发 unsubscribe，不造成 borrow/deadlock。
- empty observer set notify 是 no-op。
- `SubscriptionId` 出现在 Rust 方法签名时，Swift/Kotlin 生成类型仍是 `Int64` / `Long`。
- counter/app subscribe 仍立即推送当前 state。
- list subscribe 仍立即推送当前列表对应的 insert diffs。
- counter-list 现有状态和 diff 行为不变。

### 6.4 验收

- 基础验收命令通过，覆盖率 > 97%。
- Rust shared 代码行数减少，observer 相关重复逻辑下降。
- 订阅即时同步语义不能退化；端上 bridge 初始化仍能立刻拿到当前 state/items。
- iOS / Android package 和 app build 仍通过。
- subagent review 最多 6 轮。

## 7. Step 10: Metadata binary 简化

目标：让 SDK 作者不再手写 JSON 拼接。

### 7.1 具体改动

短期采用显式但更短的宏：

```rust
cross_kit::metadata_main!(
    AppViewModel,
    CounterViewModel,
    ListViewModel
);
```

该宏展开成 `main()`，读取每个 VM 的 `CkVmMetadata::ck_vm_metadata()` 并输出 JSON array。

后续可评估自动 registry，但本 Step 不引入自动 inventory，避免链接行为不透明。

### 7.2 新增测试

- `metadata_main!` 生成合法 JSON array。
- 空列表给出编译期或运行时清晰错误。
- 未实现 `CkVmMetadata` 的类型给出清晰编译错误。
- counter-list metadata snapshot 不变。

### 7.3 验收

- `examples/counter-list/shared/src/bin/ck_vm_metadata.rs` 降到极少样板。
- CLI iOS/Android package 仍能读取 metadata。
- 基础验收 + iOS/Android 端上验收通过。
- subagent review 最多 6 轮。

## 8. Step 11: 生成 root container，减少端上生命周期样板

目标：端上只创建一个 root container，Cross-Kit 生成代码内部维护 root/child bridge 和 close/deinit。

### 8.1 Root Graph Contract

实现 root container 前必须先定义 root graph contract，不能只靠“没有 factory 的 VM”猜 root。

配置建议：

```toml
[bindings]
root_vm = "AppViewModel"
container_name = "CrossKitSharedBridge"
```

规则：

- `root_vm` 必须对应 metadata 中存在的 VM。
- `container_name` 同时作为 Swift/Kotlin root container 类型名，后续如需平台差异再扩展。
- child VM 由 `factory.rust_type == root_vm` 识别。
- 本 Step 只支持 zero-arg child factory：
  - Rust 方法形状：`pub fn make_xxx(&self) -> Arc<ChildViewModel>`。
  - 如果 child factory 有参数，生成器必须报错，不做隐式传参。
- root constructor 参数来自 root VM 的 `new(...)`。
- 多 root、无 root、child factory 指向非 root、同名 child bridge 都必须报错。
- close 顺序：children 先 close，root 最后 close；close 必须 idempotent。
- Android generated root container 的 `close()` 必须可重复调用，不重复 unsubscribe。
- iOS root container 如果暴露 nested `ObservableObject` child，必须转发 child `objectWillChange`，否则 `@StateObject var kit` 读 `kit.counter.state` 不会稳定刷新。

### 8.2 iOS 目标

生成类似：

```swift
@MainActor
public final class CrossKitSharedBridge: ObservableObject {
    public let app: AppViewModelBridge
    public let counter: CounterViewModelBridge
    public let list: ListViewModelBridge
    private var cancellables: Set<AnyCancellable> = []

    public init(initial: Int32) {
        let app = AppViewModelBridge(initial: initial)
        self.app = app
        self.counter = CounterViewModelBridge(app: app)
        self.list = ListViewModelBridge(app: app)
        counter.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }.store(in: &cancellables)
        list.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }.store(in: &cancellables)
    }
}
```

example 改为：

```swift
@StateObject private var kit = CrossKitSharedBridge(initial: 0)
```

如果转发 `objectWillChange` 让生成代码过重，可替代方案是生成 SwiftUI property wrapper 或继续让端上用多个 `@StateObject`。本 Step 默认选择 root container 转发，保证目标用法成立。

### 8.3 Android 目标

生成类似：

```kotlin
class CrossKitSharedBridge(initial: Int) : AutoCloseable {
    val app = AppViewModelBridge(initial)
    val counter = app.makeCounterVm()
    val list = app.makeListVm()
    private var closed = false

    override fun close() {
        if (closed) return
        closed = true
        list.close()
        counter.close()
        app.close()
    }
}

@Composable
fun rememberCrossKitShared(initial: Int): CrossKitSharedBridge {
    val kit = remember(initial) { CrossKitSharedBridge(initial) }
    DisposableEffect(kit) {
        onDispose { kit.close() }
    }
    return kit
}
```

example 改为：

```kotlin
val kit = rememberCrossKitShared(initial = 0)
```

### 8.4 具体改动

- `cross-kit-codegen` 根据 metadata 中 root VM 和 child factory 关系生成 platform root container。
- `cross-kit-core` 增加 bindings/root graph config model 和 validation。
- CLI 将 `[bindings]` config 传给 iOS/Android packager。
- `cross-kit-packager-ios` 将 root container Swift 文件写入 package。
- `cross-kit-packager-android` 将 root container Kotlin 文件写入 AAR source。
- example 端上代码只使用 root container。

### 8.5 新增测试

- 多 child VM 生成稳定 root container。
- 无 child VM 时 root container 只持有 root bridge。
- Android generated helper 包含 `DisposableEffect` close。
- Android root close idempotent，不重复调用 child close。
- iOS generated root container 使用 `@MainActor`，并转发 child `objectWillChange`。
- root graph 歧义测试：无 root、多 root、unknown root、child factory 有参数、child factory 指向非 root。
- 生成文件 snapshot 或等价断言覆盖 root/child factory。

### 8.6 验收

- iOS `ContentView.swift` 不再手动创建 child bridge。
- Android `MainActivity.kt` 不再手写 `DisposableEffect` close 多个 bridge。
- iOS UI 读 `kit.counter.state` / `kit.list.items` 时更新能刷新。
- iOS package + xcodebuild 通过。
- Android package + assembleDebug 通过。
- 基础验收命令通过，覆盖率 > 97%。
- subagent review 最多 6 轮。

## 9. Step 12: Android packaging 配置收敛

目标：让 Android 普通用户只关注 AAR/Maven 依赖，不看到 Step 6 的 generated-source 细节。

### 9.1 具体改动

- `cross-kit.toml` 支持：

```toml
[android]
package_name = "com.crosskit.shared"
module_name = "crosskitshared"
package_output = "dist/android"
gradle_project_output = "dist/android/gradle-project"
targets = ["arm64-v8a", "x86_64"]
build_mode = "release"

[android.maven]
group_id = "com.crosskit"
artifact_id = "crosskitshared"
version = "0.1.0"
```

- `output` / `jni_libs_output` 迁移到 `[android.codegen]` 或只作为 debug command 默认值。
- `cross-kit-packager-android` 不再硬编码 `com.crosskit:crosskitshared:0.1.0`。
- Android example dependency 从 config 派生，保持当前值不变。
- 数据结构必须一路穿透：
  - `cross-kit-core` 新增 `AndroidMavenConfig { group_id, artifact_id, version }`，含默认值。
  - `cross-kit-cli` 从 config 映射到 packager options。
  - `cross-kit-packager-android::AndroidPackageOptions` 增加 maven 字段。
  - Gradle `group`、`version`、publication `artifactId` 从 options 写入。
  - generated POM / Gradle module metadata 与 config 一致。

### 9.2 新增测试

- Maven coordinates 从 config 写入 Gradle module 和 POM。
- 缺失 `[android.maven]` 时使用向后兼容默认值。
- CLI config mapping 覆盖 group/artifact/version。
- `AndroidPackageOptions` layout/Gradle 模板测试断言不再出现硬编码坐标。
- Step 6 `gen bridges` / `build-native` 仍能工作。
- config 路径相对 `cross-kit.toml` 解析不变。

### 9.3 验收

- Android AAR/POM 坐标与 config 一致。
- Android Studio / Gradle app 继续使用 Maven dependency。
- Android package + assembleDebug 通过。
- 基础验收命令通过，覆盖率 > 97%。
- subagent review 最多 6 轮。

## 10. Step 13: Minimal Counter Example

目标：新增最小上手 example，证明新用户只需极少 Rust/端上代码即可跑通。

### 10.1 功能

- 一个 `CounterViewModel`。
- state: `CounterState { value }`。
- actions: `increment()`、`decrement()`、`reset()`。
- iOS / Android UI 都只展示数字和三个按钮。

### 10.2 价值

- 展示 `#[cross_kit::vm_bridge(mode = "state")]` 最短写法。
- 展示 `metadata_main!`。
- 展示 iOS `@StateObject private var kit = ...`。
- 展示 Android `val kit = remember...`。

### 10.3 目录

```text
examples/minimal-counter/
  cross-kit.toml
  shared/
  ios/
  android/
```

### 10.4 验收

- shared crate 单测覆盖 increment/decrement/reset 和 metadata。
- iOS package + xcodebuild 通过。
- Android package + assembleDebug 通过。
- workspace 测试与覆盖率 > 97%。
- subagent review 最多 6 轮。

## 11. Step 14: Form Wizard Example

目标：展示 Rust 下沉表单状态、校验、跨步骤路由，端上只渲染 state。

### 11.1 功能

- 多步骤注册/资料填写：
  - Step 1: name/email。
  - Step 2: password/confirm。
  - Step 3: summary。
- Rust 负责：
  - validation。
  - button enabled。
  - error message。
  - current step / route。
- 端上负责：
  - 输入框。
  - 根据 state 显示错误。
  - 调用 `updateName` / `next` / `back`。

### 11.2 价值

- 展示 state VM 不只是 counter。
- 展示 route 由 Rust 决策。
- 展示表单错误和 derived UI state。
- 为未来 event/effect mode 提供演进基线。

### 11.3 验收

- Rust 单测覆盖有效/无效 email、密码不一致、step back/next、summary。
- iOS / Android UI build 通过。
- 端上不写重复 validation。
- workspace 覆盖率 > 97%。
- subagent review 最多 6 轮。

## 12. Step 15: Task Board Example

目标：展示 diff list、reorder、filter、批量操作和 derived counters。

### 12.1 功能

- Todo/task board：
  - add task。
  - toggle done。
  - reorder。
  - delete。
  - filter all/open/done。
- Rust 负责：
  - list diff。
  - ordering。
  - counters。
  - filter result。

### 12.2 价值

- 比 `counter-list` 更接近真实列表业务。
- 明确展示 diff list 的优势：端上不重新算列表，不手写 diff。
- 可作为 Android Recycler/LazyColumn 和 SwiftUI List 的标准示例。

### 12.3 验收

- Rust 单测覆盖 insert/update/remove/move/filter/batch。
- iOS / Android build 通过。
- diff invalid case 测试保留或新增。
- workspace 覆盖率 > 97%。
- subagent review 最多 6 轮。

## 13. Step 16: Shopping Cart Example

目标：展示 parent VM + child VM + derived totals + business rules。

### 13.1 功能

- 商品列表、购物车、优惠券、总价。
- Rust 负责：
  - cart item merge。
  - quantity validation。
  - discount rule。
  - subtotal/tax/total。
  - checkout readiness。
- 端上负责展示和按钮。

### 13.2 价值

- 展示 Cross-Kit 适合共享非平凡业务规则。
- 展示 iOS/Android 不重复实现金额计算和校验。
- 展示 root container 对多 VM 示例的价值。

### 13.3 验收

- Rust 单测覆盖边界：数量 0、库存不足、优惠券无效、金额舍入。
- iOS / Android build 通过。
- 端上代码不包含价格规则和折扣规则。
- workspace 覆盖率 > 97%。
- subagent review 最多 6 轮。

## 14. Step 17: State-Driven Long Operation Example

目标：新增一个模拟“看起来像异步”的业务 example，但不把 async 暴露成 Cross-Kit API 能力。Cross-Kit 的端上调用仍然是同步 action + state observation。

### 14.1 设计判断

Cross-Kit 当前应坚持状态驱动：

- 端上不调用 Rust `async fn`。
- codegen 不生成 Swift `async throws`。
- codegen 不生成 Kotlin `suspend`。
- 端上不需要理解 Rust runtime、task join、future polling。
- 失败、loading、cancelled、progress 都是 VM state 的一部分。

长任务、搜索、登录、刷新、上传这类场景可以在 Rust VM 内部模拟或执行异步工作，但对端上暴露的仍然只是普通方法：

```rust
pub fn submit(&self);
pub fn cancel(&self);
pub fn get_state(&self) -> SearchState;
```

端上使用：

```swift
kit.search.submit()
if kit.search.state.isLoading { ... }
if let error = kit.search.state.error { ... }
```

```kotlin
kit.search.submit()
if (kit.search.state.isLoading) { ... }
kit.search.state.error?.let { ... }
```

### 14.2 Example 候选

新增 `examples/search-refresh`：

- 输入 query。
- 点击 Search 后进入 loading。
- Rust VM 内部模拟网络搜索。
- 支持 cancel。
- 支持快速连续搜索时旧结果不能覆盖新 query。
- 支持结构化错误作为 state enum，例如 invalid input、network failure、cancelled。
- iOS/Android 端上只渲染 state，不写搜索业务规则。

### 14.3 Rust 侧模型

示例状态：

```rust
#[derive(Clone, Debug, uniffi::Record)]
pub struct SearchState {
    pub query: String,
    pub is_loading: bool,
    pub progress: i32,
    pub results: Vec<SearchResult>,
    pub error: Option<SearchError>,
}

#[derive(Clone, Debug, uniffi::Enum)]
pub enum SearchError {
    EmptyQuery,
    Network { code: i32, message: String },
    Cancelled,
}
```

示例 VM：

```rust
#[cross_kit::vm_bridge(mode = "state")]
#[uniffi::export]
impl SearchViewModel {
    pub fn update_query(&self, query: String);
    pub fn submit(&self);
    pub fn cancel(&self);
    pub fn get_state(&self) -> SearchState;
    pub fn subscribe(&self, observer: Arc<dyn SearchObserver>) -> i64;
}
```

### 14.4 实现约束

- 不新增 Cross-Kit runtime async 抽象。
- 不新增公共 `TaskId` / `TaskRegistry` 能力。
- 不把 Rust async 方法映射到端上 async API。
- example 可以用 deterministic fake worker、manual tick、测试注入 service 或内部线程模拟耗时。
- Rust 单测必须能稳定控制状态推进，不能依赖真实网络和不稳定 sleep。
- 如果使用内部线程，VM `close()` 或 `cancel()` 必须保证旧结果不会再覆盖新 state。

### 14.5 新增测试

- Rust 单测：
  - submit 后 state 进入 loading。
  - fake success 后 state 更新 results。
  - typed error 不丢失结构化字段。
  - cancel 后 state 进入 cancelled 或 idle，旧结果不能覆盖当前 state。
  - 连续 submit 时旧搜索结果不能覆盖新 query。
  - 空 query 给出 `SearchError::EmptyQuery`。
- iOS：
  - package + xcodebuild 通过。
  - XCTest 覆盖 submit/cancel/error state，使用 deterministic fake 能力时验证状态序列。
- Android：
  - package + Gradle assembleDebug 通过。
  - JVM/instrumented 可行时覆盖 submit/cancel/error state。
- 仍需基础验收、覆盖率 > 97%、subagent case 生成和最多 6 轮 review。

## 15. 暂不做的事

- 暂不引入完整 async runtime 方案。长任务 example 只展示 state-driven operation，不改变 Cross-Kit 公开调用模型。
- 暂不生成 Swift `async throws` 或 Kotlin `suspend` API。
- 暂不新增公共 `TaskHandle` / `TaskRegistry` 抽象，除非后续多个 examples 证明这是重复痛点。
- 暂不发布到 crates.io、Maven Central、CocoaPods trunk。
- 暂不删除旧兼容字段，例如 `swift_bridge`。
- 暂不把所有 examples 都做成视觉完整产品；先保证工程角色、调用体验、测试和打包闭环。
- 暂不实现 event mode，Form Wizard 先用 state route 表达；event mode 另起 step 讨论。

## 16. 已确认的默认决策

用户已确认可以按本文默认策略推进。当前默认决策如下：

1. Step 8B 的短宏语法采用：

```rust
#[cross_kit::vm_bridge(mode = "state", factory = AppViewModel::make_counter_vm)]
```

其中 `factory` 不是字符串，而是 Rust path。旧字符串字段保留兼容。

2. diff list 采用这个折中写法：

```rust
#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff = ListDiff,
    item = ListItem,
    factory = AppViewModel::make_list_vm
)]
```

也就是 state VM 尽量完全推断，diff list 显式声明 diff/item，但不再用字符串。

3. Step 9 优先做 `ObserverSet` 这种轻量 helper，不先做更激进的 `StateStore` / `StateSubject` 抽象，避免把业务状态机框死。

4. Step 10 先做 `metadata_main!(...)` 显式列表，不做自动 registry。自动 registry 更省代码，但链接行为和可解释性更复杂，不作为第一版。

5. Step 11 生成 root container 的名字用 `{PackageName}Bridge`。当前 example 会是 `CrossKitSharedBridge`。

6. 新 examples 按这个顺序推进：
   - `minimal-counter`
   - `form-wizard`
   - `task-board`
   - `shopping-cart`

7. examples 都要求三端完整，也就是每个 example 都包含 Rust shared、iOS App、Android App。否则不能证明端上库依赖体验真的舒服。

8. 长任务、搜索、登录等“异步形态”仍按状态驱动表达：端上调用同步 action，Rust VM 通过 state 推送 loading/progress/result/error/cancelled。默认不生成 Swift `async throws` 或 Kotlin `suspend`，也不新增公共 async runtime 能力。
