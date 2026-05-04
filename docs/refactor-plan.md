# Cross-Kit Refactor Plan

本文档用于在大规模重构前明确 Cross-Kit 的产品边界、工程角色、分阶段执行顺序和每一步验收标准。目标是避免一次性搬完目录导致方向跑偏。每个阶段必须单独 review、单独验证、单独决定是否进入下一步。

## 0. 当前结论

本文后续使用以下术语：

- Rust SDK：用户用 Rust 编写的 shared crate，也就是业务状态机、VM、domain logic 所在层。
- 端上：iOS、Android 等宿主 App 层。
- 端上库：iOS/Android 可直接依赖的产物。端上不应该感知 Rust FFI 细节、UniFFI 生成细节或 native library 装配细节。

Cross-Kit 不是单一 demo 工程，而应该同时包含四种角色：

- 一个可被 Rust 用户依赖的公开 crate。
- 一组可被端上直接依赖的库产物：iOS Swift Package / XCFramework，Android AAR / Maven artifact。
- 一个 CLI 工具链，用于生成 bridge、打包端上库、初始化模板。
- 一组 examples，用来展示 Rust shared crate、iOS App、Android App 如何依赖 Cross-Kit 开发。

当前仓库的问题是这些角色混在一起：

- `example/shared` 直接依赖 `crates/ck-vm-macros`，还没有依赖一个统一的 `cross-kit` crate。
- `tools/ck-swift-packager` 是工具，但没有纳入统一 CLI。
- `example/ios` 依赖 `example/shared/dist/CrossKitShared`，这个目录是生成产物，不应该进 git；长期应由 CLI 重新生成端上可依赖库。
- `example/android` 已有手写 bridge 雏形，但缺少 UniFFI Kotlin binding 和 `.so`，Android 还不是可验收闭环。
- `old-example` 是 SigSong 历史项目和旧工具参考，占用体量大，不应继续留在这个仓库里，也不应进入 workspace。

## 1. 最终工程角色

### 1.1 Public Runtime Crate: `cross-kit`

这是用户在自己的 Rust SDK 中真正依赖的 crate。

职责：

- 暴露稳定的 Rust API 和 trait，例如 `CkVmMetadata`、VM metadata 类型、observer/subscription 辅助类型。
- 重新导出 proc macro，例如 `cross_kit::vm_bridge`。
- 尽量保持轻量，避免绑定 iOS/Android 打包逻辑。
- 与 UniFFI 协作，但不替代 UniFFI。

非职责：

- 不直接调用 `xcodebuild`、`cargo ndk`、Gradle。
- 不生成 Swift/Kotlin 文件。
- 不包含 example 业务逻辑。

Rust SDK 目标用法：

```toml
[dependencies]
cross-kit = { path = "../../crates/cross-kit" }
uniffi = "0.29"
```

```rust
use cross_kit::{vm_bridge, CkVmMetadata};

#[vm_bridge(
    mode = "state",
    bridge = "CounterViewModelBridge",
    state = "CounterState",
    observer = "CounterObserver",
    observer_method = "on_state"
)]
#[uniffi::export]
impl CounterViewModel {
    pub fn get_state(&self) -> CounterState;
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64;
    pub fn unsubscribe(&self, id: i64);
    pub fn increment(&self) -> CounterState;
}
```

### 1.2 Generated iOS Library

这是端上 iOS 代码直接依赖的库，不是开发者手动维护的源码目录。

职责：

- 提供 SwiftPM package。
- 封装 UniFFI Swift binding、Swift bridge、XCFramework。
- 让 SwiftUI 侧只面对 `AppViewModelBridge`、`CounterViewModelBridge` 等 Swift API。
- 隐藏 Rust FFI、header、modulemap、native library 链接细节。

端上目标用法：

```swift
import CrossKitShared

@StateObject private var app = AppViewModelBridge(initial: 0)
@StateObject private var counter: CounterViewModelBridge
```

端上不应该需要：

- 手写 `System.loadLibrary` 类似逻辑。
- 直接调用 UniFFI generated low-level API。
- 理解 Rust target triple、`.a`、header、modulemap。

生成方式：

```bash
cross-kit ios package --config examples/counter-list/cross-kit.toml
```

### 1.3 Generated Android Library

这是端上 Android 代码直接依赖的库，不是开发者手动维护的 `jniLibs` 和 generated Kotlin 目录。

职责：

- 提供 AAR 或本地 Maven artifact。
- 封装 UniFFI Kotlin binding、Compose bridge、`.so`。
- 让 Compose 侧只面对 `AppViewModelBridge`、`CounterViewModelBridge` 等 Kotlin API。
- 隐藏 Rust FFI、JNI library 装载、ABI 目录和 UniFFI generated API 细节。

端上目标用法：

```kotlin
import com.crosskit.shared.AppViewModelBridge

val appVm = remember { AppViewModelBridge(initial = 0) }
val counterVm = remember(appVm) { appVm.makeCounterBridge() }
```

端上不应该需要：

- 手写 `System.loadLibrary("cross_kit_shared")`。
- 直接管理 `jniLibs/<abi>/lib*.so`。
- 直接调用 UniFFI generated low-level API。
- 理解 Rust target、cargo-ndk、ABI matrix。

生成方式：

```bash
cross-kit android package --config examples/counter-list/cross-kit.toml
```

### 1.4 Macro Crate: `cross-kit-macros`

这是 `cross-kit` 的内部配套 proc macro crate。

职责：

- 解析 `#[vm_bridge(...)]` 标注的 Rust `impl`。
- 输出编译期 metadata。
- 做尽可能多的 contract 检查，例如 state 模式必须存在 `get_state` 和 `subscribe`。

非职责：

- 不拼 Swift 源码字符串。
- 不拼 Kotlin 源码字符串。
- 不知道 package layout。

当前 `crates/ck-vm-macros` 的 Swift 字符串生成逻辑后续应迁出到 codegen。

### 1.5 Core Crate: `cross-kit-core`

这是 CLI、codegen、packager 共享的内部模型层。

职责：

- 定义 `cross-kit.toml` 配置结构。
- 定义 VM metadata IR。
- 定义平台、target、ABI、artifact model。
- 提供路径解析、workspace/package 解析、错误类型。

核心数据结构草案：

```rust
pub struct CrossKitConfig {
    pub shared: SharedConfig,
    pub ios: Option<IosConfig>,
    pub android: Option<AndroidConfig>,
}

pub struct VmModuleMetadata {
    pub schema_version: u32,
    pub crate_name: String,
    pub lib_name: String,
    pub vms: Vec<VmMetadata>,
}

pub struct VmMetadata {
    pub rust_type: String,
    pub bridge_name: String,
    pub mode: VmMode,
    pub state_type: Option<String>,
    pub diff_type: Option<String>,
    pub list_item_type: Option<String>,
    pub observer: ObserverMetadata,
    pub factory: Option<FactoryMetadata>,
    pub methods: Vec<MethodMetadata>,
}
```

### 1.6 Codegen Crate: `cross-kit-codegen`

职责：

- 输入 `cross-kit-core` 的 metadata IR。
- 输出 Swift bridge 源码。
- 输出 Kotlin Compose bridge 源码。
- 保证生成逻辑可单测、可 snapshot test。

非职责：

- 不编译 Rust。
- 不调用 Xcode/Gradle。
- 不读取 Xcode project。

生成目标：

- Swift: `ObservableObject` bridge、`@Published state`、observer proxy、`deinit unsubscribe`。
- Kotlin: Compose state bridge、主线程回调、`close()` 取消订阅、list diff 应用。

### 1.7 iOS Packager Crate: `cross-kit-packager-ios`

由当前 `tools/ck-swift-packager` 演化而来。

职责：

- 编译 Rust 到 iOS/macOS target。
- 调用 UniFFI 生成 Swift binding。
- 生成 `XCFramework`。
- 写 SwiftPM package 或 CocoaPods podspec。
- 把 `cross-kit-codegen` 生成的 Swift bridge 放进 package source。

目标命令：

```bash
cross-kit ios package --config examples/counter-list/cross-kit.toml
```

目标输出：

```text
examples/counter-list/dist/ios/CrossKitShared/
  Package.swift
  Sources/CrossKitShared/
    cross_kit_shared.swift
    Bridges/*.swift
  CrossKitSharedFFI.xcframework
```

### 1.8 Android Packager Crate: `cross-kit-packager-android`

新增。

职责：

- 使用 `cargo ndk` 或等价流程编译 `.so`。
- 调用 UniFFI 生成 Kotlin binding。
- 调用 `cross-kit-codegen` 生成 Compose bridge。
- 组装 Android library module 或 AAR。
- 可选发布到本地 Maven 目录。

目标命令：

```bash
cross-kit android package --config examples/counter-list/cross-kit.toml
```

目标输出：

```text
examples/counter-list/dist/android/
  com.crosskit.example.shared.aar
  maven/
```

### 1.9 CLI Crate: `cross-kit-cli`

职责：

- 提供 `cross-kit` 二进制。
- 组织 `init`、`gen`、`ios package`、`android package` 等命令。
- 做用户输入校验和命令编排。

CLI 命令草案：

```bash
cross-kit init counter-list
cross-kit metadata --config cross-kit.toml
cross-kit gen bridges --config cross-kit.toml --platform ios
cross-kit ios package --config cross-kit.toml
cross-kit android package --config cross-kit.toml
```

### 1.10 Examples

examples 不是 Cross-Kit 源码的一部分，而是消费 Cross-Kit 的示范项目。

目标结构：

```text
examples/
  counter-list/
    cross-kit.toml
    shared/
      Cargo.toml
      src/lib.rs
      src/bin/ck_metadata.rs
    ios/
      CrossKitExample.xcodeproj
      CrossKitExample/
    android/
      settings.gradle.kts
      app/
```

依赖方向：

- `examples/counter-list/shared` 依赖 `crates/cross-kit`。
- `examples/counter-list/ios` 依赖 CLI 生成的 SwiftPM package。iOS 示例代码不直接接触 Rust FFI 细节。
- `examples/counter-list/android` 依赖 CLI 生成的 AAR 或本地 Maven。Android 示例代码不直接接触 Rust FFI 细节。
- examples 不能反向被 `crates/*` 依赖。

## 2. 目标目录结构

```text
Cross-Kit/
  Cargo.toml
  README.md
  LICENSE

  crates/
    cross-kit/
    cross-kit-macros/
    cross-kit-core/
    cross-kit-codegen/
    cross-kit-packager-ios/
    cross-kit-packager-android/
    cross-kit-cli/

  examples/
    counter-list/
      cross-kit.toml
      shared/
      ios/
      android/

  docs/
    architecture.md
    vm-contract.md
    cli.md
    packaging-ios.md
    packaging-android.md
    refactor-plan.md

```

`old-example` 不进入目标目录结构。Step 0 先把它移动到仓库上一层作为外部参考备份，例如：

```text
../Cross-Kit-old-example/
```

迁出后本仓库只记录 `old-example` 从主线移除，不保留 `legacy/` 目录，不进入 workspace，不进入 CI。

## 3. Git 与生成产物规则

必须进 git：

- Rust crates 源码。
- examples 源码。
- `cross-kit.toml` 示例配置。
- 文档。
- 最小必要的 Xcode/Gradle project 文件。

不能进 git：

- `dist/`
- `target/`
- `DerivedData/`
- `.gradle/`
- Android `build/`
- `.so`
- `.a`
- `.xcframework`
- `.aar`
- SwiftPM `.build/`

例外：

- 若某个 demo 在早期阶段必须临时依赖生成产物才能被 Xcode 打开，需要在阶段文档中标记为临时例外，并在后续阶段移除。

## 4. 分阶段重构计划

每个阶段都必须满足：

- 阶段开始前记录 `git status --short`。
- 阶段只解决本阶段定义的问题。
- 阶段结束后更新文档。
- 每个阶段最终形成一个独立 commit。
- 阶段实现完成后先不要提交，必须先补齐测试、跑完验证、完成独立 subagent review。
- 阶段结束后进行 review，确认并 commit 后再进入下一阶段。

每个阶段的固定执行顺序：

1. 实现本阶段代码和文档变更。
2. 补充测试：历史 case 必须继续通过，新增逻辑必须有有意义的新增 case。
3. 覆盖率检查：预期核心 Rust 代码行覆盖率超过 97%。如果某阶段没有 Rust 行覆盖率工具可用，必须说明原因并给出替代验证。
4. 运行本阶段验收命令。
5. 调用一个独立 subagent 阅读方案文档和未提交代码，做 code review。
6. 修复 review 发现的问题，并重新运行受影响测试。
7. 重复 subagent review，最多六轮。
8. 当 review 不再发现问题，且测试和覆盖率满足验收标准后，才允许 commit。
9. commit 后进入下一阶段。

subagent review 规则：

- subagent 必须阅读 `docs/refactor-plan.md` 和本阶段未提交 diff。
- review 优先级：行为 bug、contract 漏洞、端上 API 泄露 Rust 细节、生成产物误入 git、测试缺口、文档与实现不一致。
- 如果六轮 review 仍有无法解决的问题，停止执行并向用户确认。
- subagent 不负责替代本地测试，review 通过不等于测试通过。

### Step 0: 冻结现状与清理决策

目的：

在搬目录前明确当前可保留资产，避免误删历史参考或覆盖已有未提交修改。

动作：

- 记录当前 `git status --short`。
- 列出 `old-example` 中仍有参考价值的部分：
  - `old-example/cargo-swift`：可作为 iOS packager 参考。
  - `old-example/sigsong-sdk`：可作为 UniFFI + 真实业务 SDK 参考。
  - `old-example/sigsong-ios` / `old-example/sigsong-android`：可作为历史接入参考。
- 明确 `old-example/Cargo.toml` 和 `old-example/Cargo.lock` 当前被删除是否保留删除。
- 将 `old-example` 移动到仓库上一层，例如 `../Cross-Kit-old-example`。这是外部备份，不进入 git，不进入 workspace。
- 确认 `example/shared/dist` 不再作为 git 资产。

交付物：

- 一份 `docs/current-inventory.md`。
- 一份迁出清单，记录 `old-example` 已移动到哪个仓库外路径。

验收命令：

```bash
git status --short
cargo fmt --manifest-path example/shared/Cargo.toml
cargo test --manifest-path example/shared/Cargo.toml
cargo llvm-cov --manifest-path example/shared/Cargo.toml --summary-only
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

验收标准：

- 不改变源码行为。
- Rust shared 测试通过。
- Rust shared 行覆盖率超过 97%。
- iOS example 构建通过。
- `old-example` 已迁出仓库，仓库内不保留 `legacy` 目录。
- 迁出前有 inventory 文档，避免后续找不到参考来源。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。
- 本阶段最终形成一个独立 commit。

停止条件：

- 如果发现 `old-example` 中仍有无法替代的代码，仍然迁出到仓库外，但必须在 `docs/current-inventory.md` 记录具体路径和用途，不允许继续留在本仓库主线。

### Step 1: 建立 Workspace 与公开 crate 骨架

目的：

让 Cross-Kit 拥有清晰的 Rust crate 边界，先不移动 example。

动作：

- 新建顶层 `Cargo.toml` workspace。
- 新建 `crates/cross-kit`。
- 新建 `crates/cross-kit-core`。
- 将 `crates/ck-vm-macros` 迁移或重命名为 `crates/cross-kit-macros`。
- `cross-kit` 依赖并 re-export `cross-kit-macros`。
- 暂时保留 `tools/ck-swift-packager` 原位置，避免同时修改太多。
- `example/shared` 改为依赖 `cross-kit`，不直接依赖 macro crate。

预期依赖图：

```text
examples/counter-list/shared 或 example/shared
  -> cross-kit
       -> cross-kit-core
       -> cross-kit-macros
```

交付物：

- 顶层 workspace 可运行。
- `cross-kit` crate 有 README 或 crate-level docs。
- `example/shared` 仍保持当前行为。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo test --manifest-path example/shared/Cargo.toml
cargo test --manifest-path tools/ck-swift-packager/Cargo.toml
xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

验收标准：

- 全 workspace Rust 测试通过。
- 核心 Rust 行覆盖率超过 97%，或明确说明该阶段覆盖率统计边界。
- `tools/ck-swift-packager` 在迁入 workspace 之前必须通过单独测试命令。
- example/shared 不再直接引用 `ck-vm-macros`。
- iOS example 仍能构建。
- 未引入 Android 新要求。
- 本阶段新增或调整的 crate re-export 行为有测试覆盖。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- `cross-kit` 公开 API 是否足够小。
- `cross-kit-core` 是否没有平台打包逻辑。
- `cross-kit-macros` 是否仍能兼容现有 metadata。

### Step 2: 定义 VM Metadata Contract

目的：

把生成器依赖的“事实结构”稳定下来，避免继续在宏里拼目标语言代码。

动作：

- 在 `cross-kit-core` 中定义 versioned metadata IR。
- `cross-kit-macros` 输出 IR JSON，不再把 Swift code 作为核心字段。
- 保留兼容层：短期可以同时输出旧字段 `swift_code`，但标记 deprecated。
- 新增 metadata snapshot tests。
- 新增 `cross-kit metadata` 命令或临时 bin，用于输出 metadata。

交付物：

- `docs/vm-contract.md`。
- metadata schema 示例文件，例如 `fixtures/metadata/counter-list.json`。
- macro contract 单测。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo run -p cross-kit-cli -- metadata --config examples/counter-list/cross-kit.toml
```

如果 CLI 尚未接入 example，则允许临时命令：

```bash
cargo run --manifest-path example/shared/Cargo.toml --bin ck_vm_metadata
```

验收标准：

- metadata 有 `schema_version`。
- Swift/Kotlin 生成所需信息全部在 IR 中。
- IR 不包含目标语言源码作为长期 contract。
- 现有 iOS build 不回退。
- metadata schema、缺失必需方法、factory child VM、类型映射至少有新增测试覆盖。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- `state`、`diff_list`、未来 `event` 三种模式是否可表达。
- `Arc<T>`、`Option<T>`、`Vec<T>`、enum、record 映射是否明确。
- factory child VM 是否表达清晰。

### Step 3: 拆出 Swift Codegen

目的：

把当前 macro 内部的 Swift 字符串生成迁移到可测试的 codegen crate。

动作：

- 新建 `crates/cross-kit-codegen`。
- 实现 `generate_swift_bridge(metadata) -> GeneratedFileSet`。
- 从 `cross-kit-macros` 删除或废弃 Swift 生成逻辑。
- `tools/ck-swift-packager` 改为调用 `cross-kit-codegen` 生成 Swift bridge。
- 添加 Swift bridge snapshot tests。

交付物：

- Swift bridge 由 codegen 生成。
- macro 不再拥有目标语言代码模板。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/CrossKitExample.xcodeproj \
  -scheme CrossKitExample \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

若 example 尚未移动，则使用当前路径：

```bash
cargo run --manifest-path tools/ck-swift-packager/Cargo.toml -- \
  --crate-path ./example/shared \
  --package-name CrossKitShared \
  --lib-name cross_kit_shared \
  --targets ios,ios-sim,ios-sim-x86_64 \
  --lib-type static \
  --format spm \
  --swift-bridges

xcodebuild -project example/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

验收标准：

- 生成的 Swift bridge 与当前功能等价。
- iOS example 构建通过。
- `cross-kit-macros` 不再是 Swift generator。
- Swift codegen 对 state VM、diff list VM、unsubscribe、factory init、type mapping 有 snapshot 或等价断言测试。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- Swift output 是否稳定、可读、可 snapshot。
- 线程切换和 `unsubscribe` 生命周期是否保留。
- list diff 应用逻辑是否有测试。

### Step 4: iOS Packager 纳入 CLI

目的：

把当前 `ck-swift-packager` 变成 `cross-kit ios package` 的正式实现。

动作：

- 新建或迁移 `crates/cross-kit-packager-ios`。
- 新建 `crates/cross-kit-cli`。
- `cross-kit-cli` 提供 `ios package`。
- 支持 `cross-kit.toml`。
- 输出到 `dist/ios`。
- 确认 `dist/` 被 `.gitignore` 忽略。

交付物：

- `cross-kit ios package --config ...` 可用。
- `docs/packaging-ios.md`。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
rm -rf examples/counter-list/dist
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/CrossKitExample.xcodeproj \
  -scheme CrossKitExample \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
git status --short
```

验收标准：

- iOS package 可从源码重新生成。
- `dist/` 不出现在 git tracked changes。
- iOS example 不依赖仓库中已提交的生成产物。
- 原 `tools/ck-swift-packager` 可删除或变成薄 wrapper。
- CLI config 解析、缺失配置、输出路径、重复生成覆盖、错误信息至少有新增测试覆盖。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- CLI 参数是否稳定。
- `cross-kit.toml` 是否足够表达 iOS package。
- 失败错误信息是否可读。

### Step 5: 移动 examples 并确认 old-example 已迁出

目的：

清理 repo 主线目录，减少历史项目干扰。`old-example` 应该已经在 Step 0 移动到仓库上一层，本阶段只确认仓库内不再残留旧目录引用。

动作：

- 将当前 `example` 移动到 `examples/counter-list`。
- 修正 iOS project 的相对路径。
- 修正文档中的路径。
- 确认 `old-example` 已不在仓库内。
- 确认 workspace、README、docs 不再把 `old-example` 当主线工程引用。

交付物：

- 主线目录只包含 `crates`、`examples`、`docs`。
- README 以新目录为准。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo run -p cross-kit-cli -- ios package --config examples/counter-list/cross-kit.toml
xcodebuild -project examples/counter-list/ios/CrossKitExample.xcodeproj \
  -scheme CrossKitExample \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
git status --short
```

验收标准：

- 移动后 iOS 闭环仍通过。
- `old-example` 不在仓库内，不在 workspace 内。
- README 不再引导用户进入旧目录。
- 路径迁移相关脚本、config、README 示例命令都有测试或构建验证覆盖。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- 目录移动是否保持历史可读性。
- 是否误提交生成产物。
- 是否还有路径硬编码指向旧 `example/`。

### Step 6: Android Binding 与 Compose Bridge Codegen

目的：

先让 Android example 使用生成的 Kotlin binding 和 Compose bridge，不急着做 AAR。

动作：

- 在 `cross-kit-codegen` 中实现 Kotlin bridge 生成。
- UniFFI Kotlin binding 输出到 ignored generated source 目录。
- Android example 依赖生成源目录。
- 生成 `.so` 到 ignored `jniLibs` 或 Gradle configured 目录。
- 补齐 Java/JDK/Android SDK/NDK 环境要求文档。

交付物：

- `cross-kit gen bridges --platform android` 可生成 Compose bridge。
- Android example 能引用 `com.crosskit.shared`。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo run -p cross-kit-cli -- gen bridges --platform android --config examples/counter-list/cross-kit.toml
cargo run -p cross-kit-cli -- android build-native --config examples/counter-list/cross-kit.toml
cd examples/counter-list/android
./gradlew assembleDebug
```

验收标准：

- Android example 编译通过。
- 不提交 generated Kotlin binding。
- 不提交 `.so`。
- 手写 Android bridge 可以删除或只保留用户侧轻包装。
- Android 端上 API 不暴露 `System.loadLibrary`、`jniLibs`、UniFFI low-level API。
- Kotlin bridge 对 state VM、diff list VM、主线程回调、close/unsubscribe、invalid diff 至少有新增测试或可执行验证。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- Kotlin bridge API 是否贴近 Compose。
- 生命周期 `close()` 是否可控。
- 主线程回调是否明确。

### Step 7: Android AAR Packager

目的：

把 Android 从“example 可编译”推进到“可分发库”。

动作：

- 新建 `cross-kit-packager-android`。
- 生成 Android library module。
- 打包 AAR。
- 可选输出 local Maven repo。
- Android example 改为依赖 AAR 或 local Maven。

交付物：

- `cross-kit android package --config ...`。
- `docs/packaging-android.md`。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
cargo run -p cross-kit-cli -- android package --config examples/counter-list/cross-kit.toml
cd examples/counter-list/android
./gradlew clean assembleDebug
```

验收标准：

- AAR 包含 Kotlin binding、Compose bridge、`.so`。
- Android example 通过 AAR/local Maven 消费。
- `.aar` 不进 git。
- AAR package contents、ABI matrix、缺失 NDK/Gradle 错误路径至少有新增测试或脚本验证。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

Review 重点：

- ABI 选择是否合理。
- Gradle 版本和 Android plugin 版本是否固定。
- AAR 消费方式是否接近真实用户。

### Step 8: CLI Init 与模板

目的：

让 Cross-Kit 能生成一个新的可运行项目。

动作：

- 实现 `cross-kit init`。
- 模板基于 `examples/counter-list` 提炼。
- 新增端到端测试：在临时目录 init、package iOS、可选 package Android。

交付物：

- `cross-kit init my-app`。
- `docs/cli.md`。

验收命令：

```bash
cargo fmt --all
cargo test --workspace
cargo llvm-cov --workspace --summary-only
tmpdir=$(mktemp -d)
cargo run -p cross-kit-cli -- init "$tmpdir/my-app"
cargo run -p cross-kit-cli -- ios package --config "$tmpdir/my-app/cross-kit.toml"
```

验收标准：

- 新项目不依赖仓库内部相对路径。
- 模板文档能从零跑通 iOS。
- Android 模板按 Step 7 能跑通。
- init 模板路径、覆盖已有目录、非法项目名、缺失平台配置至少有新增测试覆盖。
- 核心 Rust 行覆盖率超过 97%。
- 本阶段通过最多六轮 subagent review，review 不再发现问题后再 commit。

## 5. Review Gate

每个 Step 是一个独立 commit。实现完成后先不要提交，必须完成测试、覆盖率、subagent review 和用户需要的确认后再 commit。

每个 Step 结束前必须提交以下 review 信息：

```text
Step: <编号和名称>

Changed:
- ...

Tests Added:
- ...

Verification:
- [pass/fail] cargo fmt --all
- [pass/fail] cargo test --workspace
- [pass/fail] cargo llvm-cov ... --summary-only
- [pass/fail] iOS package
- [pass/fail] iOS build
- [pass/fail] Android build

Coverage:
- Lines: <value>
- Boundary: <workspace/package list>
- Below 97% reason: <only allowed with explicit explanation>

Subagent Review:
- Round 1: <findings/fixed/no findings>
- Round 2: <findings/fixed/no findings>
- ...
- Final: <no findings / blocked after six rounds>

Generated Artifacts:
- dist ignored: yes/no
- native artifacts tracked: yes/no

Open Questions:
- ...

Decision Needed Before Next Step:
- ...
```

如果任意核心验收失败，不进入下一步。如果 subagent review 六轮仍然发现无法解决的问题，停止并向用户确认。commit message 必须包含 Step 编号，例如 `step 1: establish cross-kit workspace`。

## 6. 已确认决策

### 6.1 Public crate 名称

建议名称：`cross-kit`。

原因：

- 用户依赖时直观。
- CLI 二进制也可叫 `cross-kit`，这没有实际问题。
- Rust import 为 `cross_kit`。

具体约束：

- Rust package 名不能在同一个 workspace 内重复。因此 runtime crate package 用 `cross-kit`。
- CLI crate package 用 `cross-kit-cli`，但它生成的 binary 名称可以是 `cross-kit`。
- 对用户来说，Rust SDK 依赖 `cross-kit`，命令行也执行 `cross-kit`，语义一致。
- 代价主要是文档表达时要区分 “runtime crate `cross-kit`” 和 “CLI binary `cross-kit`”。工程上不冲突。

结论：

- 接受 runtime crate 和 CLI binary 同名。
- 不让两个 Cargo package 同名。
- 最终命名按 `cross-kit` 执行。

### 6.2 Macro 名称

宏通过 `cross_kit::` 暴露，短期使用：

```rust
#[cross_kit::vm_bridge(...)]
```

未来如果要做更高层 DSL，再新增：

```rust
#[cross_kit::vm(...)]
```

结论：

- 暂不考虑 `ck` 缩写。
- 短期宏入口使用 `cross_kit::vm_bridge`；未来可再增加 `cross_kit::vm`。

### 6.3 old-example 策略

建议：

- Step 0 先 inventory。
- 移动到仓库上一层，例如 `../Cross-Kit-old-example`。
- 不进入当前仓库。
- 不进入 workspace。
- 不保留 `legacy` 目录。

结论：

- 不直接删除文件内容，先移动到仓库外。
- 当前仓库 commit 中体现为移除 `old-example`。

### 6.4 Android 验证环境

当前本机 `./gradlew assembleDebug` 失败，原因是没有 Java Runtime。

结论：

- Android 阶段需要真实验证闭环。
- 如果缺 JDK/Android SDK/NDK，由执行 agent 直接配置环境。
- 在环境未准备好之前，不能声称 Android 闭环通过。

### 6.5 生成产物策略

建议：

- `dist/` 永远不进 git。
- iOS/Android examples 的 README 写清楚先运行 package 命令。
- CI 每次从源码生成 package。

结论：

- 接受打开 example 前需要先运行 `cross-kit ios package` 或对应平台 package 命令。

### 6.6 端上库 API 边界

建议：

- iOS 端上依赖 SwiftPM package。
- Android 端上依赖 AAR 或 Maven artifact。
- 端上只使用 generated bridge，例如 `AppViewModelBridge`。
- 端上不直接感知 UniFFI low-level API、native library 装载、Rust target、ABI、header/modulemap。

结论：

- 接受 Cross-Kit 生成的端上库作为唯一推荐入口。
- iOS example 不直接引用 UniFFI generated low-level API。
- Android example 不直接引用 UniFFI generated low-level API。

## 7. 推荐立即执行顺序

不要直接进入目录大搬迁。建议先做：

1. Step 0：冻结现状与清理决策。
2. Step 1：建立 workspace 和 `cross-kit` public crate。
3. Step 2：稳定 metadata contract。
4. Step 3：拆 Swift codegen。
5. Step 4：iOS CLI package 闭环。

只有 Step 4 通过后，再移动 example。`old-example` 在 Step 0 就迁出仓库外。这样即使目录移动产生大 diff，也有稳定的 iOS 验收链路兜底。
