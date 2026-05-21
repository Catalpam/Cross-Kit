# Cross-Kit Step 18+ Iteration Plan

本文档基于当前 `crates/`、`examples/*/shared`、iOS `ContentView.swift`、Android
`MainActivity.kt` 的实际代码，规划 Step 18 之后的迭代。目标不是继续堆 example，
而是把 Cross-Kit 的使用体验继续往“Rust 业务代码更少模板、端上代码更少桥接细节、
框架内部承担更多生命周期和同步工作”的方向推进。

## 0. 当前判断

已经做得比较好的部分：

- Rust 用户已经可以依赖公开 crate `cross-kit`，不用直接依赖内部 core/macro crate。
- `#[cross_kit::vm_bridge(mode = "state")]` 的最短写法已经可用，常见 state VM 不再需要手写
  bridge/state/observer 名称。
- `factory = AppViewModel::make_child_vm` 这种 Rust path 写法已经比字符串安全。
- iOS 业务代码基本只保留一个 `@StateObject private var kit = ...`。
- Android 业务代码基本只保留一个 `rememberCrossKit...Bridge()`。
- root container 已经统一持有 root/child bridge，并处理 Android `close()` 和 iOS child
  `objectWillChange` 转发。
- examples 已覆盖 minimal counter、form wizard、task board、shopping cart、search refresh 和
  counter-list，业务形态比最初完整很多。

仍然明显不够顺的部分：

- Rust 侧每个 state VM 仍要手写 `Mutex<State>`、`ObserverSet`、`subscribe`、`unsubscribe`、
  `locked_state`、`notify`。
- Rust 侧每个 diff-list VM 仍要手写 insert replay、diff application/生成、visible list
  snapshot，`task-board`、`shopping-cart`、`counter-list` 有重复模式。
- Step 18 pre-commit review 已修复 iOS generated bridge 暴露 `getState()` 的问题，后续需要把
  “Swift/Kotlin 都只暴露 observable state/items，不暴露 polling-shaped API” 固化成回归测试和
  文档约束。
- 示例里错误态还偏向“错误对象展示”，真实业务更应该把失败、空态、提示、弹窗、字段校验等都表达成
  state/presentation state，端上只负责渲染状态。
- Step 18 已经给 `cross-kit`、`cross-kit-core`、`cross-kit-macros` 的公开入口补上基础 rustdoc，
  并通过 `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p cross-kit -p cross-kit-core
  -p cross-kit-macros` 固化门禁。后续仍要在新增 public API 时保持同等文档密度。
- Android/iOS example 首次运行前需要先 package，IDE 体验依赖用户知道正确命令；之前 Android
  Studio 里 “Module not specified” 就属于这类工程准备不清晰的问题。
- Step 18 已新增 Android examples “可启动、可见、可交互”门禁。`scripts/check-android-examples.sh`
  会逐个 package、安装、启动、检查 native libs、检查 fatal logs、确认前台 Activity、等待关键 UI
  文本、截图像素粗检并跑 instrumentation tests；提交前还必须人工打开截图确认不是黑屏且 UI 可读。
- Step 18 已修正 `docs/cross-kit-cli.md` 里的 async guidance：Rust 业务内部可以做线程/timer/IO
  调度，但 Cross-Kit 暴露给 SwiftUI/Compose 的平台 API 仍保持 sync action + observed state/items。
- 生成代码格式可读性一般，例如 Swift method body indentation 偏深；这不影响编译，但会影响用户
  打开 generated package 时的信任感。

## 1. 设计原则

后续步骤按这些规则取舍：

- Cross-Kit 对外继续坚持 state-driven 模型：端上调用同步 action，观察 state/items，不生成
  Swift `async throws` 或 Kotlin `suspend` API。
- Rust SDK 仍然拥有业务状态机。框架可以下沉 observer/replay/diff 模板，但不把业务规则强塞进
  通用抽象。
- 默认路径要短；高级 override 可以有，但 examples 不应该展示复杂参数。
- 端上代码不应该感知 subscription id、UniFFI observer proxy、native library、root/child VM
  创建关系、close 顺序。
- 端上也不应该默认感知“错误对象”。错误通常应该先被 Rust 业务状态机转成可展示状态，例如
  `notice`、`dialog`、`field_errors`、`empty_state`、`can_retry`。typed domain error 可以保留在
  Rust 内部用于测试、重试、统计和分支逻辑，但不应该成为 SwiftUI/Compose 的默认渲染入口。
- 每一步必须小而可验收，不能一次性做完整大重构。

## 2. 三端 Review 结论

### 2.1 Rust 侧

当前 Rust business code 的核心形态是合理的：状态、业务动作、校验、diff 生成都下沉在 shared
crate 中，端上不重复业务规则。

#### 2.1.1 当前写法示例：单 state VM

`minimal-counter`、`form-wizard`、`search-refresh` 都有类似结构。以 counter 为例，业务逻辑很少，
但为了接入 Cross-Kit，仍要写 store lock、observer set、subscribe replay、unsubscribe、notify：

```rust
#[derive(uniffi::Object)]
pub struct CounterViewModel {
    state: Mutex<CounterState>,
    observers: ObserverSet<dyn CounterObserver>,
}

#[vm_bridge(mode = "state")]
#[uniffi::export]
impl CounterViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(CounterState { value: initial }),
            observers: ObserverSet::new(),
        })
    }

    pub fn increment(&self) {
        self.update_by(1);
    }

    pub fn get_state(&self) -> CounterState {
        self.locked_state()
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> SubscriptionId {
        let state = self.locked_state();
        let subscription_id = self.observers.subscribe(observer.clone());
        observer.on_state(state);
        subscription_id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.observers.unsubscribe(id);
    }
}

impl CounterViewModel {
    fn update_by(&self, delta: i32) {
        let state = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.value += delta;
            state.clone()
        };
        self.notify(state);
    }

    fn locked_state(&self) -> CounterState {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn notify(&self, state: CounterState) {
        let observers = self.observers.snapshot();
        ObserverSet::notify_snapshot(&observers, |observer| {
            observer.on_state(state.clone());
        });
    }
}
```

这里真正的业务只有 `state.value += delta`，其余基本是 Cross-Kit 接入模板。

期望改成更接近：

```rust
pub struct CounterViewModel {
    state: StateStore<CounterState, dyn CounterObserver>,
}

#[vm_bridge(mode = "state")]
#[uniffi::export]
impl CounterViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self> {
        Arc::new(Self {
            state: StateStore::new(CounterState { value: initial }),
        })
    }

    pub fn increment(&self) {
        self.state.update_notify(
            |state| state.value += 1,
            |observer, state| observer.on_state(state),
        );
    }

    pub fn get_state(&self) -> CounterState {
        self.state.read()
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> SubscriptionId {
        self.state
            .subscribe_replay(observer, |observer, state| observer.on_state(state))
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.state.unsubscribe(id);
    }
}
```

这个目标仍然不是“完全无模板”。原因是 Rust 泛型无法可靠知道 observer trait 的回调方法名，所以
`observer.on_state(state)` 先用闭包保留。这样框架下沉锁、id、snapshot、poison recovery，
业务代码仍清楚表达回调语义。

#### 2.1.2 当前写法示例：root + child VM

`task-board` 和 `shopping-cart` 已经展示了正确方向：root VM 创建 child VM，二者共享同一个 Rust
store，端上只看到 generated root container。

当前写法：

```rust
#[derive(Clone)]
struct Store {
    inner: Arc<Mutex<StoreInner>>,
    board_observers: ObserverSet<dyn TaskBoardObserver>,
    list_observers: ObserverSet<dyn TaskListObserver>,
}

#[derive(uniffi::Object)]
pub struct TaskBoardViewModel {
    store: Store,
}

#[derive(uniffi::Object)]
pub struct TaskListViewModel {
    store: Store,
}

#[vm_bridge(mode = "state")]
#[uniffi::export]
impl TaskBoardViewModel {
    pub fn make_task_list_vm(self: Arc<Self>) -> Arc<TaskListViewModel> {
        Arc::new(TaskListViewModel {
            store: self.store.clone(),
        })
    }
}

#[vm_bridge(
    mode = "diff_list",
    diff = TaskDiff,
    item = TaskItem,
    factory = TaskBoardViewModel::make_task_list_vm
)]
#[uniffi::export]
impl TaskListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn TaskListObserver>) -> SubscriptionId {
        let visible = self.store.visible_tasks();
        let id = self.store.list_observers.subscribe(observer.clone());
        if !visible.is_empty() {
            observer.on_diffs(
                visible
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| TaskDiff::Insert {
                        index: index as i64,
                        item,
                    })
                    .collect(),
            );
        }
        id
    }
}
```

这个结构的优点：

- root/child VM 共享 Rust store，业务一致性好。
- `factory = TaskBoardViewModel::make_task_list_vm` 已经让生成器知道 child 生命周期。
- 端上不需要手动创建 child bridge。

问题在于 diff-list replay 模板重复，而且每个业务都要手写 “items -> insert diffs”。

期望补一个轻量 helper 后变成：

```rust
impl TaskListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn TaskListObserver>) -> SubscriptionId {
        let visible = self.store.visible_tasks();
        let id = self.store.list_observers.subscribe(observer.clone());
        observer.on_diffs(items_as_insert_diffs::<_, TaskDiff>(&visible));
        id
    }
}
```

进一步的 `DiffListStore<T>` 暂不建议马上做，因为 `task-board` 有 filter 后的 visible list，
`shopping-cart` 有 product id / stock / position，`counter-list` 有 replay window。轻量 helper
比大抽象更稳。

#### 2.1.3 Rust 侧结论

模板仍偏多：

- `ObserverSet<dyn XObserver>` 每个 VM 都要声明。
- `subscribe` 几乎都是 `get_state/get_items -> observers.subscribe -> replay -> return id`。
- `unsubscribe` 几乎都是一行。
- state VM 的 `Mutex` 锁、poison recovery、clone 后 notify 写法重复。
- diff-list 的 initial replay 和 common diff helpers 在多个 example 中重复。

这意味着下一轮最值得做的不是新增宏语法，而是补 runtime helper，让 Rust SDK 作者少写订阅和
diff 样板。

### 2.2 iOS 侧

当前目标用法已经比较接近理想形态：

```swift
@StateObject private var kit = CrossKitShoppingCartBridge()
Text("\(kit.shoppingCart.state.totalCents)")
Button("Add") { kit.cart.addProduct(productId: id, quantity: 1) }
```

#### 2.2.1 当前业务代码示例

以 `shopping-cart` 为例，当前端上代码已经比较理想：

```swift
struct ContentView: View {
    @StateObject private var kit = CrossKitShoppingCartBridge()

    private var state: ShoppingCartState {
        kit.shoppingCart.state
    }

    var body: some View {
        VStack {
            Text("Total \(money(state.totalCents))")

            ForEach(state.products, id: \.id) { product in
                Button("Add") {
                    kit.cart.addProduct(productId: product.id, quantity: 1)
                }
            }

            ForEach(Array(kit.cart.items.enumerated()), id: \.element.productId) { _, item in
                Text("\(item.name) x\(item.quantity)")
            }
        }
    }
}
```

这说明 root container 的方向是对的：SwiftUI 只创建一个 `kit`，然后读取 state/list，调用 action。
端上没有 observer id、没有 FFI、没有 child VM factory、没有 close 顺序。

#### 2.2.2 已修复的 generated API 泄漏

Step 18 pre-commit review 之前，generated bridge 会把 `get_state` 变成 public Swift API：

```swift
@MainActor
public final class SearchViewModelBridge: ObservableObject, SearchObserver {
    @Published public private(set) var state: SearchState

    public func updateQuery(query: String) {
        vm.updateQuery(query: query)
    }

    public func getState() -> SearchState {
        return vm.getState()
    }

    public func onState(state: SearchState) {
        self.state = state
    }
}
```

这种形态会让用户同时看到两个入口：

```swift
kit.search.state      // 正确：observable state
kit.search.getState() // 不推荐：polling/FFI-shaped API
```

Step 18 已把 Swift/Kotlin 统一为隐藏 `get_state`。后续文档和测试只需要守住这个 contract：
端上业务代码读取 observable `state/items`，调用 action 方法，不直接调用 state polling API。

过滤规则应保持类似：

```rust
fn filtered_kotlin_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| method.name != "subscribe" && method.name != "new")
        .filter(|method| method.name != "unsubscribe")
        .filter(|method| method.name != "get_state")
        .collect()
}
```

Swift 侧应该和 Kotlin 对齐：

```rust
fn filtered_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| method.name != "subscribe" && method.name != "new")
        .filter(|method| method.name != "unsubscribe")
        .filter(|method| method.name != "get_state")
        .collect()
}
```

目标生成结果：

```swift
public final class SearchViewModelBridge: ObservableObject, SearchObserver {
    @Published public private(set) var state: SearchState

    public func updateQuery(query: String) { ... }
    public func submit() { ... }
    public func cancel() { ... }

    public func onState(state: SearchState) {
        self.state = state
    }
}
```

#### 2.2.3 iOS 侧结论

主要问题：

- `getState()` 泄漏问题已在 Step 18 修复；后续需要把“只读 `state/items`，不暴露 polling-shaped
  API”作为 generated API 的稳定约束。
- generated Swift source 的缩进质量不够好。
- 错误/失败展示不应该依赖 `String(describing: error)`。真实产品更适合由 Rust state 给出
  `notice`、`dialog`、`field_error`、`can_retry` 等展示状态。
- root container 目前没有显式 `close()`，靠 Swift deinit 和 bridge deinit。对大多数 SwiftUI 页面
  可以接受，但文档需要说明；如果后续要支持手动释放，应保持幂等。

### 2.3 Android 侧

当前目标用法也已经比较接近理想形态：

```kotlin
val kit = rememberCrossKitTaskBoardBridge()
Text("Open ${kit.taskBoard.state.openCount}")
LazyColumn { items(kit.taskList.items) { ... } }
```

#### 2.3.1 当前业务代码示例

以 `task-board` 为例，Compose 入口已经很短：

```kotlin
@Composable
fun CrossKitApp(modifier: Modifier = Modifier) {
    val kit = rememberCrossKitTaskBoardBridge()
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        TaskBoardScreen(
            state = kit.taskBoard.state,
            taskList = kit.taskList,
            onFilter = kit.taskBoard::setFilter,
            modifier = Modifier.padding(innerPadding)
        )
    }
}

@Composable
private fun TaskBoardScreen(
    state: TaskBoardState,
    taskList: TaskListViewModelBridge,
    onFilter: (TaskFilter) -> Unit,
) {
    Text("Open ${state.openCount}")
    LazyColumn {
        itemsIndexed(taskList.items, key = { _, task -> task.id }) { _, task ->
            Button(onClick = { taskList.toggleDone(task.id) }) {
                Text(task.title)
            }
        }
    }
}
```

这已经满足大方向：Compose 页面只拿 generated bridge，不知道 Rust observer、subscription id、native
library、child factory 或 close 顺序。

#### 2.3.2 当前生成代码示例

生成的 root container 大致是：

```kotlin
class CrossKitTaskBoardBridge() : AutoCloseable {
    val taskBoard: TaskBoardViewModelBridge = TaskBoardViewModelBridge()
    val taskList: TaskListViewModelBridge = taskBoard.makeTaskListVm()
    private var closed = false

    override fun close() {
        if (closed) return
        closed = true
        taskList.close()
        taskBoard.close()
    }
}

@Composable
fun rememberCrossKitTaskBoardBridge(): CrossKitTaskBoardBridge {
    val kit = remember(Unit) { CrossKitTaskBoardBridge() }
    DisposableEffect(kit) {
        onDispose { kit.close() }
    }
    return kit
}
```

这说明 Android lifecycle 的核心路径是正确的：`remember` 和 `DisposableEffect` 已经由 Cross-Kit
生成，业务代码不需要手写。

仍不够的是 example 首次打开体验。真实用户如果没有先运行 package/prepare，Android Studio 里会先看到
缺 module、缺 generated dependency 或 run configuration 为空，而不是直接看到可运行 app。

#### 2.3.3 错误展示示例

当前端上失败展示基本是直接渲染 error 对象：

```kotlin
state.error?.let { error ->
    Text(text = error.toString())
}
```

或 Swift：

```swift
if let error = state.error {
    Text(String(describing: error))
}
```

这适合 debug，但不适合产品 UI。更好的 example 形态是 Rust state 直接给展示状态：

```rust
pub struct SearchState {
    pub query: String,
    pub status: SearchStatus,
    pub notice: Option<SearchNotice>,
    pub can_retry: bool,
}

pub enum SearchStatus {
    Idle,
    Loading,
    Results,
    Empty,
    Failed,
}

pub enum SearchNotice {
    Inline { message: String },
    Toast { message: String },
    Dialog { title: String, message: String },
}
```

端上只渲染状态，不需要判断“这是错误还是成功”：

```kotlin
when (val notice = state.notice) {
    is SearchNotice.Inline -> Text(notice.message)
    is SearchNotice.Dialog -> ErrorDialog(notice.title, notice.message)
    is SearchNotice.Toast -> ToastHost(notice.message)
    null -> Unit
}
```

typed domain error 仍可以留在 Rust 内部：

```rust
enum SearchFailure {
    EmptyQuery,
    Network { code: i64 },
    Cancelled,
}

fn failure_to_state(failure: SearchFailure) -> SearchState {
    match failure {
        SearchFailure::EmptyQuery => SearchState {
            status: SearchStatus::Failed,
            notice: Some(SearchNotice::Inline {
                message: "Enter a query to search.".to_string(),
            }),
            can_retry: false,
            ..SearchState::default()
        },
        SearchFailure::Network { .. } => SearchState {
            status: SearchStatus::Failed,
            notice: Some(SearchNotice::Toast {
                message: "Search is temporarily unavailable.".to_string(),
            }),
            can_retry: true,
            ..SearchState::default()
        },
        SearchFailure::Cancelled => SearchState::default(),
    }
}
```

这样 Rust 测试仍能覆盖失败原因，端上只关心展示状态。

#### 2.3.4 Android 侧结论

主要问题：

- `rememberCrossKit...Bridge()` 好用，但 example 首次打开前必须先生成/publish local artifact，否则 IDE
  里会出现 module/config 不完整的体验。
- 错误展示不应默认暴露 raw error 给端上。更推荐 presentation state：inline message、toast、
  dialog、field errors、empty state、retry state。
- generated root container 的 lifecycle 已经幂等 close，但没有 `ViewModel`/lifecycle-owner 版本。
  但本阶段先不考虑非 SwiftUI/非 Compose，避免把兼容面过早扩大。

### 2.4 `on_state` / `onState` / `on_diffs` 到底是什么

`on_state` 不是端上业务代码要主动调用的 action，它是 Rust observer trait 的回调方法名。当前
metadata contract 里有：

```json
{
  "mode": "state",
  "observer": {
    "rust_type": "CounterObserver",
    "method": "on_state"
  },
  "state_type": "CounterState"
}
```

Rust shared crate 里会定义 observer trait：

```rust
#[uniffi::export(with_foreign)]
pub trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}
```

生成器读取 `observer.method = "on_state"` 后，在 Swift 里生成 `onState`，在 Kotlin 里生成
`onState`。这些 platform observer proxy 是框架生成的，业务 UI 不应该手写：

```swift
public final class CounterViewModelBridge: ObservableObject, CounterObserver {
    @Published public private(set) var state: CounterState

    public func onState(state: CounterState) {
        self.state = state
    }
}
```

```kotlin
class CounterViewModelBridge : CounterObserver, AutoCloseable {
    var state by mutableStateOf(vm.getState())
        private set

    override fun onState(state: CounterState) {
        this.state = state
    }
}
```

`on_diffs` 是 diff-list VM 的同类回调，不是和 `on_state` 对立，而是另一种同步模型：

```rust
#[uniffi::export(with_foreign)]
pub trait TaskListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<TaskDiff>);
}
```

生成 bridge 会把 diff 应用到平台侧 list：

```kotlin
override fun onDiffs(diffs: List<TaskDiff>) {
    items = applyDiffs(items, diffs)
}
```

当前职责边界是：

- Rust 业务代码负责在状态变化后调用 `observer.on_state(state)` 或 `observer.on_diffs(diffs)`。
- Cross-Kit 宏负责把 callback 名称写进 metadata，并可在默认写法里推断 `state -> on_state`、
  `diff_list -> on_diffs`。
- Cross-Kit codegen 负责生成 Swift/Kotlin observer proxy，把 callback 转成 `@Published state`、
  Compose `mutableStateOf` 或 list items。
- 未来 `StateStore` / diff helper 可以减少 Rust 手写 `subscribe/replay/notify` 模板，但 callback
  本身仍是 Rust observer contract 的一部分，不是端上需要感知的 API。

## 3. 执行规则

每个 Step 都必须满足：

- Step 开始前确认工作区状态，不覆盖用户未提交改动。
- 每个 Step 一个独立 commit。
- Step 完成后先不提交，先补测试。
- 补测试必须包含执行者自己设计的 case。
- 提交前必须调起独立 subagent，让它提出至少 5 条、至多 20 条有意义测试 case，尽量覆盖 Rust、
  iOS、Android。
- 筛选并补足 case 后，运行相关测试、全仓测试和覆盖率。
- Rust workspace 行覆盖率要求继续 `> 97%`。
- 从 Step 18 开始，Android example 运行门禁是提交前必跑项：所有 Android examples 至少要 package、
  assemble、启动无崩溃，并通过首屏可见 UI 的 instrumentation/smoke test。不能只用“Gradle 编译通过”
  作为 Android 可用性的判断。
- 提交前再调独立 subagent 阅读文档和未提交 diff 做 review。
- review 后修复问题并重跑相关测试，最多 6 轮；6 轮仍不能收敛再停下来问用户。
- 测试、覆盖率、review 都通过后才 commit。
- commit 后确认工作区干净，再进入下一步。

## 4. Step 18: Android example 启动门禁 + Public API 文档化

目标：先解决 example “编译通过但启动崩溃/黑屏不可用”的问题，并把这个检查变成后续所有 Step 的硬门禁。
同时补框架公开 API 的文档债。Cross-Kit 是给 Rust SDK 作者、iOS/Android 端上开发者共同使用的框架，
public struct/function/macro 如果没有文档注释，用户只能读源码猜 contract，这会直接影响后续重构的
可维护性。

### 4.1 当前代码形态

#### 4.1.1 Android example 启动问题

当前 Android example 不能只看 Gradle 编译是否通过。真实问题发生在运行期：

```text
java.lang.UnsatisfiedLinkError:
Native library (com/sun/jna/android-aarch64/libjnidispatch.so) not found
```

根因是 generated Android library 如果依赖 JNA jar 而不是 JNA Android AAR，APK 里不会包含
`libjnidispatch.so`。APK 里应该能看到：

```text
lib/arm64-v8a/libcross_kit_task_board_shared.so
lib/arm64-v8a/libjnidispatch.so
lib/x86_64/libcross_kit_task_board_shared.so
lib/x86_64/libjnidispatch.so
```

修完 JNA 后还会暴露另一个运行期问题：

```text
UnexpectedUniFFICallbackError:
NullPointerException: MutableState.setValue(...) on a null object reference
```

这个问题来自 generated Kotlin bridge 初始化顺序：

```kotlin
class TaskBoardViewModelBridge : TaskBoardObserver {
    private val vm: TaskBoardViewModel = TaskBoardViewModel()
    private val observerId: Long = vm.subscribe(this)

    var state: TaskBoardState by mutableStateOf(vm.getState())
        private set
}
```

Rust `subscribe` 会立即 replay 当前 state，此时 `state` 还没初始化，所以 callback 写入会崩。目标顺序是：

```kotlin
class TaskBoardViewModelBridge : TaskBoardObserver {
    private val vm: TaskBoardViewModel = TaskBoardViewModel()

    var state: TaskBoardState by mutableStateOf(vm.getState())
        private set

    private val observerId: Long = vm.subscribe(this)
}
```

diff-list bridge 也是同样规则，必须先初始化 `items`：

```kotlin
val items: SnapshotStateList<TaskItem> = mutableStateListOf()
private val observerId: Long = vm.subscribe(this)
```

最后还需要验证“用户真实看得到页面”。Android 门禁不能只接受 `uiautomator dump` 有节点；UI hierarchy
只能作为辅助定位，不能替代视觉验收。如果截图全黑、纯色、只有系统背景，或者看不出首屏内容，就算
accessibility tree 里能读到节点也必须判失败。自动脚本负责采集截图和做粗粒度像素检查，但 Step 18
提交前还必须由执行者打开截图人工确认：标题、核心状态和主要按钮/输入框都肉眼可见，不能只看脚本
返回码。这个 Step 必须切换到稳定模拟器、软件渲染或 Managed Device，直到截图里能看到 example 页面
后才能通过。

#### 4.1.2 Public API 文档缺口

`crates/cross-kit/src/lib.rs` 里 `ObserverSet` 有一段概览，但方法缺少 rustdoc：

```rust
impl<O: ?Sized> ObserverSet<O> {
    pub fn new() -> Self { ... }

    pub fn subscribe(&self, observer: Arc<O>) -> SubscriptionId { ... }

    pub fn unsubscribe(&self, id: SubscriptionId) -> bool { ... }

    pub fn notify(&self, mut f: impl FnMut(&Arc<O>)) { ... }

    pub fn snapshot(&self) -> Vec<Arc<O>> { ... }
}
```

`crates/cross-kit-core/src/lib.rs` 里很多 config/metadata struct 的 public fields 也没有逐字段说明：

```rust
pub struct AndroidMavenConfig {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub artifact_id_explicit: bool,
}

impl VmMetadata {
    pub fn validate(&self) -> Result<(), MetadataValidationError> { ... }
}
```

这些 API 已经被 CLI、packager、example metadata binary 和未来外部用户依赖。缺少文档会带来几个问题：

- `ObserverSet::notify` 和 `snapshot` 的锁语义不清楚，用户不知道 callback 里能不能 unsubscribe。
- `subscribe` 是否 replay 当前 state 不清楚。实际上 `ObserverSet` 只登记 observer，不 replay；
  replay 是 VM/store 层职责。
- `AndroidMavenConfig.artifact_id_explicit` 是反序列化辅助字段，不应该被误认为普通用户配置。
- `VmMetadata.validate` 的失败时机和 macro/codegen 关系不清楚。

### 4.2 目标代码形态

#### 4.2.1 Android 启动门禁目标

新增一个可重复运行的 Android example smoke gate，建议先放在脚本或 xtask/CLI 子命令中：

```bash
./scripts/check-android-examples.sh
```

脚本做的事情必须覆盖所有 examples：

```bash
examples=(
  minimal-counter
  counter-list
  form-wizard
  search-refresh
  shopping-cart
  task-board
)

for example in "${examples[@]}"; do
  cargo run -p cross-kit-cli -- android package --config "examples/$example/cross-kit.toml"
  (cd "examples/$example/android" && ./gradlew assembleDebug connectedDebugAndroidTest)
done
```

每个 example 的运行期检查至少包含：

```bash
adb logcat -c
adb shell am force-stop com.example.crosskit_example_android
adb shell am start -n com.example.crosskit_example_android/.MainActivity
sleep 2

adb logcat -d -t 300 | rg -i \
  "FATAL EXCEPTION|AndroidRuntime|UnsatisfiedLinkError|UnexpectedUniFFICallbackError|NullPointerException" \
  && exit 1

adb shell dumpsys activity activities | rg "topResumedActivity=.*com.example.crosskit_example_android"
adb shell uiautomator dump /sdcard/window.xml
adb pull /sdcard/window.xml /tmp/cross-kit-window.xml
```

每个 example 还要有至少一个“首屏可见内容”的 Compose/instrumentation 断言，例如：

```kotlin
@Test
fun appLaunchesAndShowsInitialContent() {
    composeRule.onNodeWithText("Task Board").assertIsDisplayed()
    composeRule.onNodeWithTag("task.add").assertIsDisplayed()
}
```

对视觉黑屏的门禁：

```bash
adb exec-out screencap -p > /tmp/cross-kit-screen.png
```

验收时不能接受“Activity 在前台但截图全黑”。如果 `uiautomator dump` 有 UI 节点但截图全黑，仍然视为
失败，必须先修模拟器/渲染配置或换用稳定 Managed Device。这个门禁的目标是用户能在 Android Studio
里肉眼看到 example 页面，而不是仅自动化树能读到文本。

实际修复方向不能只改脚本。当前 examples 使用 edge-to-edge + `Scaffold` 时，曾出现“accessibility tree
有 Text/Button，截图只有黑色窗口和状态栏”的情况。Android example 根部应该显式提供绘制背景和内容色：

```kotlin
setContent {
    CrosskitexampleandroidTheme {
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background,
            contentColor = MaterialTheme.colorScheme.onBackground
        ) {
            CrossKitApp()
        }
    }
}
```

这个约束的目的不是美化页面，而是避免端上用户第一次打开 example 时看到黑屏，同时让 screenshot gate
有真实首屏内容可以验收。

已知必须防回归的 codegen/packager 断言：

```rust
#[test]
fn android_packager_depends_on_jna_android_aar() {
    let gradle = write_gradle_project(...);
    assert!(gradle.contains("net.java.dev.jna:jna:5.14.0@aar"));
}

#[test]
fn kotlin_state_bridge_initializes_state_before_subscribe() {
    let code = generate_kotlin_bridge_source(&state_metadata(), "com.crosskit.shared").unwrap();
    assert!(
        code.find("var state: CounterState by mutableStateOf(vm.getState())").unwrap()
            < code.find("private val observerId: Long = vm.subscribe(this)").unwrap()
    );
}

#[test]
fn kotlin_list_bridge_initializes_items_before_subscribe() {
    let code = generate_kotlin_bridge_source(&list_metadata(), "com.crosskit.shared").unwrap();
    assert!(
        code.find("val items: SnapshotStateList<ListItem> = mutableStateListOf()").unwrap()
            < code.find("private val observerId: Long = vm.subscribe(this)").unwrap()
    );
}
```

#### 4.2.2 Public API 文档目标

给公开 API 补文档性质注释，不做简单代码翻译，而说明 contract、职责和边界：

```rust
/// Registers an observer and returns the stable subscription id used to remove it later.
///
/// This method only stores the observer. It does not replay the current state or emit
/// initial list diffs; replay belongs to the VM or store layer because only that layer
/// knows which callback should be invoked and which snapshot should be sent.
pub fn subscribe(&self, observer: Arc<O>) -> SubscriptionId;
```

```rust
/// Returns a snapshot of currently subscribed observers.
///
/// Notification code should call callbacks from this snapshot instead of while holding
/// the internal observer lock. This allows a callback to unsubscribe or add another
/// subscription without deadlocking the observer set.
pub fn snapshot(&self) -> Vec<Arc<O>>;
```

```rust
/// Target-independent VM description emitted by `#[cross_kit::vm_bridge]`.
///
/// Code generators consume this metadata to build SwiftUI/Compose friendly bridges.
/// The metadata should describe Rust-facing VM methods and observer callbacks, not
/// target-language source code.
pub struct VmMetadata {
    /// Metadata schema version. Generators must reject versions they do not support.
    pub schema_version: u32,
    /// Rust object type annotated with `#[cross_kit::vm_bridge]`.
    pub rust_type: String,
    ...
}
```

对 public macro 也补文档：

```rust
/// Generates a metadata binary `main` function for the listed VM types.
///
/// The binary prints a JSON array consumed by `cross-kit-cli ios package` and
/// `cross-kit-cli android package`.
metadata_main!(CounterViewModel, TaskListViewModel);
```

### 4.3 文件改动范围

- `crates/cross-kit-packager-android/src/lib.rs`
  - Android generated library 必须依赖 JNA AAR：`net.java.dev.jna:jna:5.14.0@aar`。
  - POM/Gradle metadata 测试要断言 JNA 的 AAR 类型能传递到 example app。
- `crates/cross-kit-codegen/src/lib.rs`
  - Kotlin state bridge：先初始化 `mutableStateOf(vm.getState())`，再 `vm.subscribe(this)`。
  - Kotlin diff-list bridge：先初始化 `mutableStateListOf()`，再 `vm.subscribe(this)`。
  - 增加顺序断言，避免之后重构 formatter 时回退。
- `scripts/check-android-examples.sh` 或等价 CLI/xtask
  - package + assemble + install/launch/logcat + connectedDebugAndroidTest。
  - 覆盖全部 6 个 Android examples。
- `examples/*/android/app/src/androidTest/...`
  - 每个 example 至少有一个 launch smoke test，断言首屏关键文本/按钮可见。
- `examples/*/android/app/src/main/java/.../ui/theme/Theme.kt`
  - 主题必须稳定可见，不能依赖会让示例黑底黑字的系统动态色。
- `crates/cross-kit/src/lib.rs`
  - `SubscriptionId`
  - `ObserverSet` 和所有 public methods
  - `CkVmMetadata::ck_vm_metadata`
  - `metadata_json`
  - `metadata_main!`
  - Step 20 引入的 `StateStore`、Step 21 引入的 diff helper 也要同步带 rustdoc。
- `crates/cross-kit-core/src/lib.rs`
  - `CrossKitConfig`、`SharedConfig`、`BindingsConfig`、`IosConfig`、`AndroidConfig`
  - `AndroidMavenConfig`
  - `VmMetadata`、`ObserverMetadata`、`FactoryMetadata`、`MethodMetadata`、`ArgMetadata`
  - `VmMode`
  - `MetadataValidationError`
  - 所有 public constructors/parsers/validators。
- `crates/cross-kit-codegen/src/lib.rs`
  - 如果存在 public data model 或 public generation entrypoint，也补 rustdoc。
  - 如果只是 crate 内部函数，暂不强制。
- generated Swift/Kotlin bridge templates
  - root container class、VM bridge class、`rememberCrossKit...Bridge()`、public action methods
    生成简短 doc comment/KDoc。
  - 注释说明“业务代码调用 action，读取 observable state/items；不要直接管理 observer/subscription”。

### 4.4 文档质量标准

每个 public item 至少回答这些问题中的相关部分：

- 它给谁用：Rust SDK 作者、metadata binary、codegen、packager，还是 generated bridge。
- 它负责什么，不负责什么。
- 生命周期或线程语义是什么，例如是否持锁调用 callback、是否 replay 初始 state、是否幂等。
- 出错时机是什么，例如 metadata validation 何时失败。
- 对 SwiftUI/Compose 生成有什么影响。

不接受这种无效注释：

```rust
/// Unsubscribes an observer.
pub fn unsubscribe(&self, id: SubscriptionId) -> bool;
```

接受这种说明 contract 的注释：

```rust
/// Removes a previously registered observer.
///
/// Returns `true` when the id existed. Calling this more than once with the same id is
/// allowed and returns `false` after the first removal, which lets generated platform
/// bridges implement idempotent `close()`.
pub fn unsubscribe(&self, id: SubscriptionId) -> bool;
```

generated platform API 也要像主流框架一样给用户明确入口：

```swift
/// SwiftUI-facing root bridge for the Search Refresh example.
///
/// Hold this type with `@StateObject` and read child bridges from its public properties.
/// The root bridge owns subscription lifetimes for all generated child bridges.
public final class CrossKitSearchRefreshBridge: ObservableObject { ... }
```

```kotlin
/**
 * Compose-facing root bridge for the Search Refresh example.
 *
 * Prefer creating this through [rememberCrossKitSearchRefreshBridge] so native
 * subscriptions are closed when the composable leaves composition.
 */
class CrossKitSearchRefreshBridge : AutoCloseable { ... }
```

### 4.5 验收

- Android smoke gate：
  - 所有 6 个 examples 都重新执行 `cross-kit-cli android package`。
  - 所有 6 个 examples 都执行 `./gradlew assembleDebug connectedDebugAndroidTest`。
  - 至少在一个稳定 AVD/Managed Device 上逐个安装并启动，logcat 无：
    `FATAL EXCEPTION`、`AndroidRuntime`、`UnsatisfiedLinkError`、
    `UnexpectedUniFFICallbackError`、`NullPointerException`。
  - 每个 example 的 instrumentation test 断言首屏关键 UI 可见。
  - 截图不能是全黑、纯色或只有系统背景；即使 UI hierarchy 有节点也不能放过。必须固定一个截图可见的
    AVD/Managed Device/软件渲染配置，保证用户肉眼能看到 example 页面。
  - 提交前必须打开脚本保存的每个 example 截图人工 review，确认不是“脚本能读到节点但用户看不到内容”。
  - APK 解包能看到 `libjnidispatch.so` 和对应 `libcross_kit_*.so`。
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p cross-kit -p cross-kit-core`
- 如果 `cross-kit-codegen` 有 public API：`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p cross-kit-codegen`
- `cargo test -p cross-kit`
- `cargo test -p cross-kit-core`
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 5. Step 19: Generated API polish 和文档一致性复查

> Step 18 pre-commit review 已经把本节最明确的两个问题提前修掉：
> Swift generated bridge 不再暴露 `getState()`，`docs/cross-kit-cli.md` 也不再把 Rust
> VM async IO 描述为 Cross-Kit 默认公开模型。后续 Step 19 只保留 generated API
> polish 和文档一致性复查，不再重复这两个已完成项。

目标：复查当前 review 已经明确的问题是否完全收敛，避免继续在错误 API 上叠功能。本 Step 不改业务行为，
只收紧 generated public API 和文档表达。

### 5.1 当前代码形态

Swift/Kotlin 生成器都应该用同一类过滤规则决定哪些 Rust 方法暴露到端上 bridge。Step 18 后，
`subscribe`、`unsubscribe`、constructor/factory 细节和 state polling 方法都不应该成为端上业务
入口：

```rust
fn filtered_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| method.name != "subscribe")
        .filter(|method| method.name != "unsubscribe")
        .filter(|method| method.name != "new")
        .filter(|method| method.name != "get_state")
        .collect()
}
```

因此端上应该看到的是 observable property + intent-style action：

```swift
public final class SearchViewModelBridge: ObservableObject, SearchObserver {
    @Published public private(set) var state: SearchState

    public func updateQuery(query: String) {
        vm.updateQuery(query: query)
    }
}
```

Kotlin 侧也应该保持同一模型：

```kotlin
class SearchViewModelBridge(private val vm: SearchViewModel) : SearchObserver {
    var state by mutableStateOf(vm.getState())
        private set

    init {
        vm.subscribe(this)
    }

    fun updateQuery(query: String) {
        vm.updateQuery(query)
    }
}
```

文档也要和这个模型一致：Rust 可以在业务内部调度线程、timer、DB 或网络模拟，但 Cross-Kit 生成给
SwiftUI/Compose 的公开 API 仍然是同步 action + observed state/items，不把 Rust async 直接映射成
Swift `async throws` 或 Kotlin `suspend`。

### 5.2 目标代码形态

Swift 和 Kotlin 使用同一套 internal method 过滤规则：

```rust
fn is_bridge_internal_method(method: &MethodMetadata) -> bool {
    matches!(
        method.name.as_str(),
        "new" | "subscribe" | "unsubscribe" | "get_state"
    )
}

fn filtered_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| !is_bridge_internal_method(method))
        .collect()
}
```

目标 Swift generated bridge：

```swift
public final class SearchViewModelBridge: ObservableObject, SearchObserver {
    @Published public private(set) var state: SearchState

    public func updateQuery(query: String) { ... }
    public func submit() { ... }
    public func tick() { ... }
    public func cancel() { ... }

    public func onState(state: SearchState) {
        self.state = state
    }
}
```

目标文档表达：

```md
Cross-Kit 对外 API 保持 state-driven：端上调用同步 action，观察 generated bridge 的
state/items。Rust 内部可以使用线程、runtime、任务队列或平台服务，但这些异步细节不直接映射成
Swift async throws 或 Kotlin suspend API。
```

### 5.3 文件改动范围

- `crates/cross-kit-codegen/src/lib.rs`
  - 增加 shared internal method predicate。
  - Swift/Kotlin filtering 共用规则或至少保持一致。
  - 增加/调整单测：Swift generated bridge 不包含 `public func getState`。
- `docs/cross-kit-cli.md`
  - 替换过期 async 建议。
- 可选：如果改动很小，顺手修 Swift method indentation；如果需要重写 formatter，拆到 Step 22。

### 5.4 新增测试示例

```rust
#[test]
fn generated_swift_state_bridge_hides_get_state() {
    let code = generate_swift_bridge_source(&state_metadata()).unwrap();
    assert!(!code.contains("public func getState"));
    assert!(code.contains("@Published public private(set) var state"));
}
```

### 5.5 验收

- `cargo test -p cross-kit-codegen`
- `cargo test -p cross-kit-packager-ios`
- `cargo run -p cross-kit-cli -- ios package --config examples/search-refresh/cross-kit.toml`
- `rg "public func getState" examples/search-refresh/dist/ios` 无命中。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- `cargo llvm-cov --workspace --exclude cross-kit-packager-ios --summary-only` 总行覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 6. Step 20: Rust state VM runtime helper

目标：减少单 state VM 中重复的 `Mutex + ObserverSet + subscribe/replay/notify` 模板。先只迁移
`minimal-counter`、`form-wizard`、`search-refresh`，不碰多 VM store。

### 6.1 当前代码形态

现在 `search-refresh` 这类 VM 要自己维护状态锁、observer set、notify snapshot：

```rust
pub struct SearchViewModel {
    state: Mutex<StoreState>,
    observers: ObserverSet<dyn SearchObserver>,
}

impl SearchViewModel {
    fn mutate(&self, mutate: impl FnOnce(&mut StoreState)) -> SearchState {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        mutate(&mut state);
        state.to_search_state()
    }

    fn notify(&self, state: SearchState) {
        let observers = self.observers.snapshot();
        ObserverSet::notify_snapshot(&observers, |observer| {
            observer.on_state(state.clone());
        });
    }
}
```

### 6.2 目标 API

在 `crates/cross-kit/src/lib.rs` 增加轻量 helper：

```rust
pub struct StateStore<S, O: ?Sized> {
    state: Arc<Mutex<S>>,
    observers: ObserverSet<O>,
}

impl<S: Clone, O: ?Sized> StateStore<S, O> {
    pub fn new(state: S) -> Self;
    pub fn read(&self) -> S;
    pub fn update(&self, mutate: impl FnOnce(&mut S)) -> S;
    pub fn update_notify(
        &self,
        mutate: impl FnOnce(&mut S),
        notify: impl FnMut(&Arc<O>, S),
    ) -> S;
    pub fn subscribe_replay(
        &self,
        observer: Arc<O>,
        replay: impl FnOnce(&Arc<O>, S),
    ) -> SubscriptionId;
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool;
}
```

迁移后 `minimal-counter` 接近：

```rust
pub struct CounterViewModel {
    state: StateStore<CounterState, dyn CounterObserver>,
}

impl CounterViewModel {
    pub fn increment(&self) {
        self.state.update_notify(
            |state| state.value += 1,
            |observer, state| observer.on_state(state),
        );
    }

    pub fn get_state(&self) -> CounterState {
        self.state.read()
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> SubscriptionId {
        self.state
            .subscribe_replay(observer, |observer, state| observer.on_state(state))
    }
}
```

注意：这个 Step 不要求宏自动生成 `subscribe/unsubscribe`。原因是 observer method 可能是 `on_state`、
`on_diffs` 或业务自定义名，先用闭包显式表达，风险更低。

### 6.3 文件改动范围

- `crates/cross-kit/src/lib.rs`
  - 新增 `StateStore`。
  - 新增 runtime tests。
- `examples/minimal-counter/shared/src/lib.rs`
  - 从 `Mutex + ObserverSet` 迁移到 `StateStore`。
- `examples/form-wizard/shared/src/lib.rs`
  - 从 `Mutex + ObserverSet` 迁移到 `StateStore`。
- `examples/search-refresh/shared/src/lib.rs`
  - 只迁移 observer/state lock 模板；`ActiveSearch` 和 derived `SearchState` 逻辑保留。

### 6.4 新增测试示例

```rust
#[test]
fn state_store_replays_current_state_on_subscribe() {
    let store = StateStore::<CounterState, dyn CounterObserver>::new(CounterState { value: 7 });
    let observer = RecordingObserver::new();

    let id = store.subscribe_replay(observer.clone(), |observer, state| {
        observer.on_state(state);
    });

    assert_eq!(id, 1);
    assert_eq!(observer.states(), vec![CounterState { value: 7 }]);
}
```

### 6.5 验收

- `cargo test -p cross-kit`
- `cargo test -p minimal-counter-shared --lib --tests`
- `cargo test -p form-wizard-shared --lib --tests`
- `cargo test -p search-refresh-shared --lib --tests`
- Package/build minimal-counter iOS + Android。
- Package/build search-refresh iOS + Android。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 7. Step 21: Rust diff-list runtime helper

目标：减少 diff-list VM 的 initial replay 和 common diff construction 模板。先做 helper，不做大而全的
`DiffListStore`。

### 7.1 当前代码形态

`task-board` initial replay：

```rust
pub fn subscribe(&self, observer: Arc<dyn TaskListObserver>) -> SubscriptionId {
    let visible = self.store.visible_tasks();
    let id = self.store.list_observers.subscribe(observer.clone());
    if !visible.is_empty() {
        observer.on_diffs(
            visible
                .into_iter()
                .enumerate()
                .map(|(index, item)| TaskDiff::Insert {
                    index: index as i64,
                    item,
                })
                .collect(),
        );
    }
    id
}
```

`shopping-cart` initial replay 几乎一样，只是 diff enum 名不同。

### 7.2 目标 API

```rust
pub trait InsertDiff<Item>: Clone {
    fn insert(index: i64, item: Item) -> Self;
}

pub fn items_as_insert_diffs<Item, Diff>(items: &[Item]) -> Vec<Diff>
where
    Item: Clone,
    Diff: InsertDiff<Item>,
{
    items
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, item)| Diff::insert(index as i64, item))
        .collect()
}
```

业务 enum 实现：

```rust
impl InsertDiff<TaskItem> for TaskDiff {
    fn insert(index: i64, item: TaskItem) -> Self {
        TaskDiff::Insert { index, item }
    }
}
```

迁移后：

```rust
pub fn subscribe(&self, observer: Arc<dyn TaskListObserver>) -> SubscriptionId {
    let visible = self.store.visible_tasks();
    let id = self.store.list_observers.subscribe(observer.clone());
    observer.on_diffs(items_as_insert_diffs::<_, TaskDiff>(&visible));
    id
}
```

### 7.3 文件改动范围

- `crates/cross-kit/src/lib.rs`
  - 新增 `InsertDiff` 和 `items_as_insert_diffs`。
- `examples/task-board/shared/src/lib.rs`
  - `impl InsertDiff<TaskItem> for TaskDiff`。
  - 迁移 subscribe replay。
- `examples/shopping-cart/shared/src/lib.rs`
  - `impl InsertDiff<CartItem> for CartDiff`。
  - 迁移 subscribe replay。
- 可选：`counter-list` 因为有 replay window，更复杂，除非 helper 能无风险接入，否则留到后续。

### 7.4 新增测试示例

```rust
#[test]
fn items_as_insert_diffs_preserves_order_and_indexes() {
    let diffs = items_as_insert_diffs::<_, TaskDiff>(&items);
    assert_eq!(diffs[0], TaskDiff::Insert { index: 0, item: items[0].clone() });
    assert_eq!(diffs[1], TaskDiff::Insert { index: 1, item: items[1].clone() });
}
```

### 7.5 验收

- `cargo test -p cross-kit`
- `cargo test -p task-board-shared --lib --tests`
- `cargo test -p shopping-cart-shared --lib --tests`
- task-board iOS/Android package/build。
- shopping-cart iOS/Android package/build。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 8. Step 22: 生成 action ergonomics 和方法过滤规则

目标：让 generated platform bridge 的 public API 更像业务 API，而不是 Rust FFI 镜像。Step 19 先修
`get_state`；Step 22 做系统化 public API contract 和生成代码可读性。

### 8.1 当前代码形态

当前 Swift/Kotlin method generation 主要是机械映射：

```rust
fn format_swift_method(method: &MethodMetadata) -> String {
    let swift_name = to_swift_method_name(&method.name);
    let ret_type = map_type_to_swift(&method.return_type);
    let call = if ret_type == "Void" {
        format!("vm.{}({})", swift_name, call_args)
    } else {
        format!("return vm.{}({})", swift_name, call_args)
    };
    format!(
        "public func {swift_name}({args}){ret_sig} {{\n        {call}\n    }}\n\n",
    )
}
```

生成出来缩进不专业：

```swift
public func updateQuery(query: String) {
        vm.updateQuery(query: query)
    }
```

### 8.2 目标代码形态

统一 internal method 规则：

```rust
fn is_bridge_internal_method(method: &MethodMetadata) -> bool {
    matches!(
        method.name.as_str(),
        "new" | "subscribe" | "unsubscribe" | "get_state"
    ) || method.name.starts_with("__cross_kit_")
}
```

生成 Swift 可读格式：

```swift
public func updateQuery(query: String) {
    vm.updateQuery(query: query)
}
```

生成 Kotlin 可读格式：

```kotlin
fun updateQuery(query: String) {
    vm.updateQuery(query)
}
```

对 return value 的规则写进文档：

```rust
// 推荐：action 更新 state，由端上观察 state。
pub fn submit(&self) { ... }

// 允许：确有同步查询或命令结果时返回值。
pub fn request_summary(&self) -> bool { ... }
```

### 8.3 文件改动范围

- `crates/cross-kit-codegen/src/lib.rs`
  - internal method predicate。
  - Swift/Kotlin formatter 清理。
  - public action return 规则测试。
- `docs/vm-contract.md`
  - 明确哪些方法由 bridge 消费，哪些方法作为 public action 暴露。

### 8.4 新增测试示例

```rust
#[test]
fn generated_swift_methods_are_formatted_as_public_actions() {
    let code = generate_swift_bridge_source(&state_metadata()).unwrap();
    assert!(code.contains("public func incrementBy(deltaValue: Int32) -> CounterState"));
    assert!(!code.contains("\n            vm."));
}
```

### 8.5 验收

- `cargo test -p cross-kit-codegen`
- `cargo test -p cross-kit-packager-ios`
- `cargo test -p cross-kit-packager-android`
- Regenerate/package counter-list、minimal-counter、search-refresh，确认端上 build 通过。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 9. Step 23: Presentation state contract

目标：不引入 async/throws，也不把 raw error 作为端上默认概念。examples 要展示推荐写法：
Rust 业务状态机把失败、空态、提示、弹窗、字段校验、可重试等都转成 presentation state，
SwiftUI/Compose 只渲染 state。

### 9.1 当前代码形态

Rust state 现在偏向直接暴露 typed error：

```rust
pub enum SearchError {
    EmptyQuery,
    Network { code: i64, message: String },
    Cancelled,
}

pub struct SearchState {
    pub error: Option<SearchError>,
}
```

端上直接 stringify：

```swift
if let error = state.error {
    Text(String(describing: error))
}
```

```kotlin
state.error?.let { error ->
    Text(text = error.toString())
}
```

这会让端上承担两个不该承担的职责：

- 决定 error 应该显示成 inline text、toast、dialog、empty state 还是 retry affordance。
- 把 domain/debug error 转成用户可理解文案。

### 9.2 目标代码形态

Rust 对外 state 改成展示状态：

```rust
pub struct SearchState {
    pub query: String,
    pub status: SearchStatus,
    pub notice: Option<SearchNotice>,
    pub can_submit: bool,
    pub can_cancel: bool,
    pub can_retry: bool,
}

pub enum SearchStatus {
    Idle,
    Loading,
    Results,
    Empty,
    Failed,
}

pub enum SearchNotice {
    Inline { message: String },
    Toast { message: String },
    Dialog { title: String, message: String },
}
```

端上只根据展示状态渲染：

```swift
switch state.notice {
case .inline(let message):
    Text(message)
case .toast(let message):
    ToastBanner(message: message)
case .dialog(let title, let message):
    ErrorDialog(title: title, message: message)
case nil:
    EmptyView()
}
```

```kotlin
when (val notice = state.notice) {
    is SearchNotice.Inline -> Text(notice.message)
    is SearchNotice.Toast -> ToastBanner(notice.message)
    is SearchNotice.Dialog -> ErrorDialog(notice.title, notice.message)
    null -> Unit
}
```

typed domain error 可以保留为 Rust 内部类型，不必通过 UniFFI 暴露给平台：

```rust
enum SearchFailure {
    EmptyQuery,
    NetworkUnavailable,
    Cancelled,
}

fn apply_failure(state: &mut SearchState, failure: SearchFailure) {
    match failure {
        SearchFailure::EmptyQuery => {
            state.status = SearchStatus::Failed;
            state.notice = Some(SearchNotice::Inline {
                message: "Enter a query to search.".to_string(),
            });
            state.can_retry = false;
        }
        SearchFailure::NetworkUnavailable => {
            state.status = SearchStatus::Failed;
            state.notice = Some(SearchNotice::Toast {
                message: "Search is temporarily unavailable.".to_string(),
            });
            state.can_retry = true;
        }
        SearchFailure::Cancelled => {
            state.status = SearchStatus::Idle;
            state.notice = None;
            state.can_retry = false;
        }
    }
}
```

form wizard 同理，不暴露 `ValidationError` 给端上，而是暴露字段状态：

```rust
pub struct FormWizardState {
    pub email: String,
    pub email_error: Option<String>,
    pub password: String,
    pub password_error: Option<String>,
    pub can_continue: bool,
}
```

shopping-cart 同理，不暴露 `CartError` 给端上，而是暴露 cart presentation：

```rust
pub struct ShoppingCartState {
    pub checkout_enabled: bool,
    pub checkout_notice: Option<CartNotice>,
    pub stock_warnings: Vec<StockWarning>,
}
```

### 9.3 typed error 到底还需不需要

你的理解基本是对的：对 SwiftUI/Compose 来说，“错误”多数时候不是独立概念，而是状态的一种展示形态。
端上不应该为了显示 UI 去理解 Rust domain error。

typed error 仍然有价值，但主要在 Rust 内部或高级调试场景：

- 测试：Rust 单测可以断言 `SearchFailure::EmptyQuery` 被转成正确 presentation state。
- 业务分支：不同失败原因可能决定 `can_retry`、是否清空结果、是否保留输入。
- analytics/debug：内部可以记录错误码、失败类型、原始 message。
- 未来高级 API：如果某些 SDK 用户确实需要 structured diagnostics，可以作为可选字段或 debug stream，
  但不是 Step 23 的默认路径。

因此 Step 23 不新增 `#[derive(CrossKitError)]`，也不生成 Swift `Error` / Kotlin exception。

### 9.4 为什么不做框架级错误协议

暂不做：

```rust
#[derive(CrossKitError)]
pub enum CartError { ... }
```

原因：错误展示牵涉本地化、多语言、产品文案、analytics code、debug message 和展示形态。框架现在没有
足够案例证明哪种 contract 最稳。先在 examples 中示范“失败被 Rust 下沉为 presentation state”更符合
当前阶段。

### 9.5 文件改动范围

- `examples/search-refresh/shared/src/lib.rs`
  - 用 `SearchStatus` / `SearchNotice` / `can_retry` 替换端上直接展示 raw error。
  - 如果保留 typed failure，只作为 Rust 内部类型。
  - 单测覆盖每种 failure 到 presentation state 的映射。
- `examples/search-refresh/ios/.../ContentView.swift`
  - 使用 `state.notice` / `state.status`。
- `examples/search-refresh/android/.../MainActivity.kt`
  - 使用 `state.notice` / `state.status`。
- `examples/shopping-cart/shared/src/lib.rs`
  - 用 `checkout_notice`、`stock_warnings`、`checkout_enabled` 表达失败展示。
- shopping-cart iOS/Android 同步使用 presentation state。

### 9.6 新增测试示例

```rust
#[test]
fn empty_query_becomes_inline_notice_without_retry() {
    vm.submit();
    let state = vm.get_state();
    assert_eq!(state.status, SearchStatus::Failed);
    assert_eq!(
        state.notice,
        Some(SearchNotice::Inline {
            message: "Enter a query to search.".to_string(),
        })
    );
    assert!(!state.can_retry);
}

#[test]
fn network_failure_becomes_retryable_toast_notice() {
    vm.update_query("rust".to_string());
    vm.simulate_network_failure();
    let state = vm.get_state();
    assert_eq!(state.status, SearchStatus::Failed);
    assert_eq!(
        state.notice,
        Some(SearchNotice::Toast {
            message: "Search is temporarily unavailable.".to_string(),
        })
    );
    assert!(state.can_retry);
}
```

### 9.7 验收

- search-refresh Rust/iOS/Android tests。
- shopping-cart Rust/iOS/Android tests。
- Package/build 两个 examples。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 10. Step 24: Example bootstrap / doctor

目标：让用户打开 iOS/Android example 前知道如何生成端上依赖，减少 IDE 配置困惑。

### 10.1 当前体验

用户如果直接打开 Android Studio，可能看到 Run config 没有 module 或依赖未生成。现在 README 需要用户手动知道：

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/search-refresh/cross-kit.toml

cd examples/search-refresh/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug testDebugUnitTest assembleDebugAndroidTest
```

### 10.2 目标 CLI

```bash
cargo run -p cross-kit-cli -- example prepare --name search-refresh
```

输出示例：

```text
Preparing example: search-refresh
[1/4] Building iOS Swift package ... ok
[2/4] Building Android AAR ... ok
[3/4] Verifying Android debug build ... ok
[4/4] Verifying metadata binary ... ok

Open iOS project:
  examples/search-refresh/ios/crosskit-example-ios.xcodeproj

Open Android project:
  examples/search-refresh/android
```

Doctor：

```bash
cargo run -p cross-kit-cli -- doctor --config examples/search-refresh/cross-kit.toml
```

输出示例：

```text
Cross-Kit doctor
Rust toolchain: ok
Metadata binary: ok (ck_search_refresh_metadata)
iOS targets: missing ios-sim-x86_64 target
Android SDK: ok
JDK: ok (/opt/homebrew/opt/openjdk@21)
cargo-ndk: missing

Suggested fix:
  cargo install cargo-ndk
  rustup target add aarch64-linux-android x86_64-linux-android
```

### 10.3 文件改动范围

- `crates/cross-kit-cli/src/main.rs`
  - 新增 `example prepare` subcommand。
  - 新增 `doctor` subcommand。
  - 内部复用现有 `ios package` / `android package` 逻辑。
- `README.md`
  - examples 运行入口改为先 `example prepare`。
- 各 example README
  - 保留底层命令，但推荐 prepare。

### 10.4 新增测试示例

```rust
#[test]
fn example_prepare_resolves_known_example_config() {
    let plan = example_prepare_plan("search-refresh").unwrap();
    assert_eq!(plan.config_path, PathBuf::from("examples/search-refresh/cross-kit.toml"));
    assert!(plan.steps.contains(&PrepareStep::IosPackage));
    assert!(plan.steps.contains(&PrepareStep::AndroidPackage));
}
```

### 10.5 验收

- CLI plan/config tests。
- Mocked process tests，避免单测真实跑 Xcode/Gradle。
- 手动或集成验收至少跑：
  - `cargo run -p cross-kit-cli -- example prepare --name minimal-counter`
  - `cargo run -p cross-kit-cli -- doctor --config examples/search-refresh/cross-kit.toml`
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 11. Step 25: SwiftUI/Compose lifecycle polish

目标：只打磨当前已经支持的 SwiftUI/Compose 默认路径，暂不扩展非 SwiftUI/非 Compose。重点是把 root
container、child bridge、close/deinit、objectWillChange 转发的 contract 写清楚并补测试。

### 11.1 当前代码形态

Kotlin root container 已有 close：

```kotlin
class CrossKitSearchRefreshBridge() : AutoCloseable {
    val search: SearchViewModelBridge = SearchViewModelBridge()
    private var closed = false

    override fun close() {
        if (closed) return
        closed = true
        search.close()
    }
}
```

Swift root container 当前没有显式 close：

```swift
@MainActor
public final class CrossKitSearchRefreshBridge: ObservableObject {
    public let search: SearchViewModelBridge
    private var cancellables: Set<AnyCancellable> = []
}
```

### 11.2 目标代码形态

SwiftUI 默认继续是：

```swift
@StateObject private var kit = CrossKitSearchRefreshBridge()
```

但 generated root container 需要把 lifecycle contract 表达清楚，并尽量提供显式幂等 close。具体写法实现
前要先验证 Swift `@MainActor` + `deinit` 约束：

```swift
@MainActor
public final class CrossKitSearchRefreshBridge: ObservableObject {
    public let search: SearchViewModelBridge
    private var cancellables: Set<AnyCancellable> = []
    private var closed = false

    public func close() {
        if closed { return }
        closed = true
        cancellables.removeAll()
        search.close()
    }
}
```

Android Compose 默认继续是：

```kotlin
@Composable
fun CrossKitApp() {
    val kit = rememberCrossKitSearchRefreshBridge()
    SearchScreen(state = kit.search.state, onSubmit = kit.search::submit)
}
```

生成的 remember helper 继续负责 `DisposableEffect` 里的 close：

```kotlin
@Composable
fun rememberCrossKitSearchRefreshBridge(): CrossKitSearchRefreshBridge {
    val kit = remember(Unit) { CrossKitSearchRefreshBridge() }
    DisposableEffect(kit) {
        onDispose { kit.close() }
    }
    return kit
}
```

暂不生成 Android `ViewModel` helper，也不写 UIKit / AppKit / XML View 示例。等 SwiftUI/Compose 路径稳定后，
如果真实用户需要再单独开 step。

### 11.3 文件改动范围

- `crates/cross-kit-codegen/src/lib.rs`
  - Swift root container close generation。
  - Swift child bridge close generation，如当前只有 deinit unsubscribe，需确认是否增加 public close。
- `docs/vm-contract.md` 或新 lifecycle doc。
- Android 只补 Compose remember/DisposableEffect 的 generated-source assertion，除非发现现有 close 不足。

### 11.4 新增测试示例

```rust
#[test]
fn generated_swift_root_container_has_idempotent_close() {
    let code = generate_swift_root_container(&metadata, &bindings).unwrap().files[0].contents;
    assert!(code.contains("private var closed = false"));
    assert!(code.contains("public func close()"));
    assert!(code.contains("if closed { return }"));
}
```

### 11.5 验收

- `cargo test -p cross-kit-codegen`
- counter-list iOS package/build，确认 multi-child root 编译。
- minimal-counter/search-refresh iOS package/build，确认 single-child/root 编译。
- Android codegen tests 保持 close idempotent。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 12. Step 26: More complete app-level example, only if still needed

当前 examples 已经不少，不建议立刻继续堆。只有当 Step 20-25 的 helper 需要更真实场景验证时才做。

### 12.1 候选 example

`examples/session-feed`：

```text
Root VM: SessionFeedViewModel
  state: SessionFeedState
  actions: login, logout, refresh, cancelRefresh, setFilter, clearRoute

Child VM: FeedListViewModel
  mode: diff_list
  item: FeedItem
  diff: FeedDiff
  factory: SessionFeedViewModel::make_feed_list_vm
```

Rust state：

```rust
pub struct SessionFeedState {
    pub session: Option<SessionUser>,
    pub filter: FeedFilter,
    pub is_refreshing: bool,
    pub refresh_progress: i64,
    pub notice: Option<SessionFeedNotice>,
    pub can_retry: bool,
    pub route: Option<SessionRoute>,
}
```

端上目标：

```swift
@StateObject private var kit = CrossKitSessionFeedBridge()

if let user = kit.sessionFeed.state.session {
    FeedList(items: kit.feedList.items)
} else {
    LoginForm(onSubmit: kit.sessionFeed.login)
}
```

```kotlin
val kit = rememberCrossKitSessionFeedBridge()
if (kit.sessionFeed.state.session != null) {
    FeedList(items = kit.feedList.items)
} else {
    LoginForm(onSubmit = kit.sessionFeed::login)
}
```

### 12.2 这个 example 用来验证什么

- root state + child diff-list。
- state-driven login/refresh/cancel，不暴露 async。
- presentation state。
- route/effect state。
- StateStore 和 diff helper 在更真实场景下是否够用。

### 12.3 暂不做的原因

如果 Step 20-25 已经通过现有 examples 证明能力，这个 example 可以不做。新增 example 会增加维护成本，
应作为验证抽象不足的工具，而不是为了数量继续堆。

### 12.4 验收

- Rust shared tests 覆盖 login/logout/refresh/cancel/filter/error/route。
- metadata fixture。
- iOS package/build/test。
- Android package/build/test。
- `cargo test --workspace --jobs 2 -- --test-threads=1`
- 覆盖率 `> 97%`
- subagent case 生成 + subagent review 通过。

## 13. Clarify 结论

我建议下一轮顺序按正常依赖推进：

1. Step 18：Android example 启动门禁 + Public API 文档化。
2. Step 19：复查 generated API polish 和文档一致性。
3. Step 20：做 `StateStore`，先迁移单 state VM examples。
4. Step 21：做轻量 diff-list helper，迁移 list replay 重复代码。
5. Step 22：整理 generated bridge public API。
6. Step 23：presentation state contract。
7. Step 24：example prepare / doctor。
8. Step 25：只打磨 SwiftUI/Compose lifecycle。
9. Step 26：如仍有必要，再补更完整 app-level example。

### 13.1 `StateStore` 闭包方案 vs 宏全自动方案

两个方案的区别：

```rust
// 方案 A：StateStore 下沉锁、observer、snapshot、poison recovery；
// Rust 业务代码仍显式说明怎么通知 observer。
self.state.update_notify(
    |state| state.value += 1,
    |observer, state| observer.on_state(state),
);
```

```rust
// 方案 B：宏尝试自动生成 subscribe/unsubscribe/get_state/notify。
#[vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn increment(&self) {
        self.state.value += 1;
    }
}
```

方案 A 优点：

- 风险小，和当前 UniFFI/export impl 结构兼容。
- callback 语义清楚，`on_state`、`on_diffs`、业务自定义 callback 都能覆盖。
- 容易测试，错误也更像普通 Rust 代码。
- 不要求宏重写用户 impl 或强制 VM 内部字段命名。

方案 A 不足：

- 仍有少量闭包模板。
- `subscribe/unsubscribe/get_state` 暂时还要业务代码保留。

方案 B 优点：

- 业务代码最短，理论上接近“只写状态和 action”。
- 订阅生命周期更彻底地下沉到框架。

方案 B 不足：

- proc macro 需要理解/改写 VM 存储结构，和 UniFFI export 的交互风险高。
- 多 root、child factory、derived state、diff-list replay、custom observer method 都会让规则复杂化。
- 出错时可能变成难懂的宏展开错误。

结论：Step 20 先做方案 A。等 `StateStore` 在多个 examples 里稳定后，再考虑是否值得做方案 B。

### 13.2 typed error 是否还需要

端上默认不需要 typed error。SwiftUI/Compose 应该渲染 presentation state：

```rust
pub struct SearchState {
    pub status: SearchStatus,
    pub notice: Option<SearchNotice>,
    pub can_retry: bool,
}
```

typed error 可以保留在 Rust 内部：

```rust
enum SearchFailure {
    EmptyQuery,
    NetworkUnavailable,
}
```

只有这些场景才考虑把 typed diagnostics 暴露出去：

- SDK 用户明确需要稳定错误码做日志/埋点。
- 端上确实需要基于错误类型接入平台能力，例如系统权限、支付失败、账号风控。
- debug build 或开发者工具需要展示原始错误。

这些都不是当前 examples 的主路径，所以 Step 23 不做框架级 typed error。
