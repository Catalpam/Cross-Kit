//! Target code generators for Cross-Kit metadata.

use cross_kit_core::{BindingsConfig, MetadataValidationError, MethodMetadata, VmMetadata, VmMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFileSet {
    pub files: Vec<GeneratedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    InvalidMetadata(MetadataValidationError),
    InvalidRootGraph(String),
    UnsupportedMode(VmMode),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadata(err) => write!(f, "{err}"),
            Self::InvalidRootGraph(err) => write!(f, "{err}"),
            Self::UnsupportedMode(mode) => write!(f, "unsupported bridge mode: {mode:?}"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<MetadataValidationError> for CodegenError {
    fn from(value: MetadataValidationError) -> Self {
        Self::InvalidMetadata(value)
    }
}

pub fn generate_swift_bridge(metadata: &VmMetadata) -> Result<GeneratedFileSet, CodegenError> {
    let contents = generate_swift_bridge_source(metadata)?;
    Ok(GeneratedFileSet {
        files: vec![GeneratedFile {
            path: format!("{}.swift", metadata.bridge_name),
            contents,
        }],
    })
}

pub fn generate_swift_bridge_source(metadata: &VmMetadata) -> Result<String, CodegenError> {
    validate_swift_bridge_metadata(metadata)?;
    match metadata.mode {
        VmMode::State => Ok(generate_state_bridge(metadata)),
        VmMode::DiffList => Ok(generate_list_bridge(metadata)),
        VmMode::Event | VmMode::Unknown => Err(CodegenError::UnsupportedMode(metadata.mode)),
    }
}

pub fn generate_kotlin_bridge(
    metadata: &VmMetadata,
    package_name: &str,
) -> Result<GeneratedFileSet, CodegenError> {
    let contents = generate_kotlin_bridge_source(metadata, package_name)?;
    Ok(GeneratedFileSet {
        files: vec![GeneratedFile {
            path: format!(
                "{}/{}.kt",
                package_name.replace('.', "/"),
                metadata.bridge_name
            ),
            contents,
        }],
    })
}

pub fn generate_kotlin_bridge_source(
    metadata: &VmMetadata,
    package_name: &str,
) -> Result<String, CodegenError> {
    validate_swift_bridge_metadata(metadata)?;
    match metadata.mode {
        VmMode::State => Ok(generate_kotlin_state_bridge(metadata, package_name)),
        VmMode::DiffList => Ok(generate_kotlin_list_bridge(metadata, package_name)),
        VmMode::Event | VmMode::Unknown => Err(CodegenError::UnsupportedMode(metadata.mode)),
    }
}

pub fn generate_swift_root_container(
    metadatas: &[VmMetadata],
    bindings: &BindingsConfig,
) -> Result<GeneratedFileSet, CodegenError> {
    let graph = RootGraph::build(metadatas, bindings)?;
    let contents = generate_swift_root_container_source(&graph);
    Ok(GeneratedFileSet {
        files: vec![GeneratedFile {
            path: format!("{}.swift", graph.container_name),
            contents,
        }],
    })
}

pub fn generate_kotlin_root_container(
    metadatas: &[VmMetadata],
    bindings: &BindingsConfig,
    package_name: &str,
) -> Result<GeneratedFileSet, CodegenError> {
    let graph = RootGraph::build(metadatas, bindings)?;
    let contents = generate_kotlin_root_container_source(&graph, package_name);
    Ok(GeneratedFileSet {
        files: vec![GeneratedFile {
            path: format!(
                "{}/{}.kt",
                package_name.replace('.', "/"),
                graph.container_name
            ),
            contents,
        }],
    })
}

struct RootGraph<'a> {
    container_name: String,
    root: &'a VmMetadata,
    children: Vec<&'a VmMetadata>,
}

impl<'a> RootGraph<'a> {
    fn build(
        metadatas: &'a [VmMetadata],
        bindings: &BindingsConfig,
    ) -> Result<RootGraph<'a>, CodegenError> {
        if bindings.root_vm.trim().is_empty() {
            return Err(CodegenError::InvalidRootGraph(
                "[bindings].root_vm must not be empty".to_string(),
            ));
        }
        if bindings.container_name.trim().is_empty() {
            return Err(CodegenError::InvalidRootGraph(
                "[bindings].container_name must not be empty".to_string(),
            ));
        }
        validate_cross_platform_identifier("[bindings].container_name", &bindings.container_name)?;
        if metadatas.is_empty() {
            return Err(CodegenError::InvalidRootGraph(
                "root graph requires at least one VM metadata entry".to_string(),
            ));
        }
        for metadata in metadatas {
            validate_swift_bridge_metadata(metadata)?;
        }

        let matching_roots = metadatas
            .iter()
            .filter(|metadata| metadata.rust_type == bindings.root_vm)
            .collect::<Vec<_>>();
        let root = match matching_roots.as_slice() {
            [root] => *root,
            [] => {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "root VM '{}' was not found in metadata",
                    bindings.root_vm
                )));
            }
            _ => {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "root VM '{}' is ambiguous in metadata",
                    bindings.root_vm
                )));
            }
        };
        if root.factory.is_some() {
            return Err(CodegenError::InvalidRootGraph(format!(
                "root VM '{}' must not be created by a factory",
                root.rust_type
            )));
        }

        let mut bridge_names = std::collections::BTreeSet::new();
        let mut property_names = std::collections::BTreeMap::new();
        for metadata in metadatas {
            if !bridge_names.insert(metadata.bridge_name.as_str()) {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "bridge name '{}' is used by more than one VM",
                    metadata.bridge_name
                )));
            }
            if bindings.container_name == metadata.bridge_name {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "[bindings].container_name '{}' conflicts with generated bridge '{}'",
                    bindings.container_name, metadata.bridge_name
                )));
            }
            for generated_name in generated_platform_type_names(metadata) {
                if bindings.container_name == generated_name {
                    return Err(CodegenError::InvalidRootGraph(format!(
                        "[bindings].container_name '{}' conflicts with generated type '{}' from VM '{}'",
                        bindings.container_name, generated_name, metadata.rust_type
                    )));
                }
            }
            let property = property_name(&metadata.rust_type);
            validate_cross_platform_identifier(
                &format!("root container property for VM '{}'", metadata.rust_type),
                &property,
            )?;
            if is_reserved_root_container_property(&property) {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "root container property '{}' for VM '{}' is reserved by generated code",
                    property, metadata.rust_type
                )));
            }
            if let Some(previous) =
                property_names.insert(property.clone(), metadata.rust_type.as_str())
            {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "VM '{}' and VM '{}' both generate root container property '{}'",
                    previous, metadata.rust_type, property
                )));
            }
        }

        let mut children = Vec::new();
        for metadata in metadatas {
            if std::ptr::eq(metadata, root) {
                continue;
            }
            let Some(factory) = &metadata.factory else {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "non-root VM '{}' must be created by root VM '{}'",
                    metadata.rust_type, root.rust_type
                )));
            };
            if factory.rust_type != root.rust_type {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "child VM '{}' factory points to '{}', expected root VM '{}'",
                    metadata.rust_type, factory.rust_type, root.rust_type
                )));
            }
            if factory.bridge_name != root.bridge_name {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "child VM '{}' factory bridge '{}' does not match root bridge '{}'",
                    metadata.rust_type, factory.bridge_name, root.bridge_name
                )));
            }
            let method = root
                .methods
                .iter()
                .find(|method| method.name == factory.method)
                .ok_or_else(|| {
                    CodegenError::InvalidRootGraph(format!(
                        "root VM '{}' is missing child factory method '{}'",
                        root.rust_type, factory.method
                    ))
                })?;
            if !method.args.is_empty() {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "child factory method '{}.{}' must not accept arguments",
                    root.rust_type, factory.method
                )));
            }
            if owned_arc_return_type(&method.return_type) != Some(metadata.rust_type.as_str()) {
                return Err(CodegenError::InvalidRootGraph(format!(
                    "child factory method '{}.{}' must return Arc<{}>",
                    root.rust_type, factory.method, metadata.rust_type
                )));
            }
            children.push(metadata);
        }

        Ok(RootGraph {
            container_name: bindings.container_name.clone(),
            root,
            children,
        })
    }
}

fn generate_swift_root_container_source(graph: &RootGraph<'_>) -> String {
    let root_property = property_name(&graph.root.rust_type);
    let (ctor_sig, ctor_call) = root_constructor_for_container(graph.root);

    let child_decls = graph
        .children
        .iter()
        .map(|child| {
            format!(
                "    public let {}: {}",
                property_name(&child.rust_type),
                child.bridge_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let child_inits = graph
        .children
        .iter()
        .map(|child| {
            format!(
                "        self.{property} = {bridge}(app: {root_property})",
                property = property_name(&child.rust_type),
                bridge = child.bridge_name,
                root_property = root_property
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let forwards = std::iter::once(graph.root)
        .chain(graph.children.iter().copied())
        .map(|metadata| {
            format!(
                "        {property}.objectWillChange.sink {{ [weak self] _ in self?.objectWillChange.send() }}.store(in: &cancellables)",
                property = property_name(&metadata.rust_type)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"// Generated by cross-kit-codegen.
import Combine
import Foundation

@MainActor
public final class {container}: ObservableObject {{
    public let {root_property}: {root_bridge}
{child_decls}
    private var cancellables: Set<AnyCancellable> = []

    {ctor_sig} {{
        let {root_property} = {root_ctor}
        self.{root_property} = {root_property}
{child_inits}
{forwards}
    }}
}}
"#,
        container = graph.container_name,
        root_property = root_property,
        root_bridge = graph.root.bridge_name,
        child_decls = indent_optional(&child_decls, 0),
        ctor_sig = ctor_sig,
        root_ctor = ctor_call,
        child_inits = indent_optional(&child_inits, 0),
        forwards = forwards,
    )
}

fn generate_kotlin_root_container_source(graph: &RootGraph<'_>, package_name: &str) -> String {
    let root_property = property_name(&graph.root.rust_type);
    let (ctor_sig, ctor_call) = root_kotlin_constructor_for_container(graph.root);
    let child_decls = graph
        .children
        .iter()
        .map(|child| {
            format!(
                "    val {}: {} = {}.{}()",
                property_name(&child.rust_type),
                child.bridge_name,
                root_property,
                to_kotlin_method_name(&child.factory.as_ref().unwrap().method)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let child_closes = graph
        .children
        .iter()
        .rev()
        .map(|child| format!("        {}.close()", property_name(&child.rust_type)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"// Generated by cross-kit-codegen.
package {package_name}

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember

class {container}{ctor_sig} : AutoCloseable {{
    val {root_property}: {root_bridge} = {root_ctor}
{child_decls}
    private var closed = false

    override fun close() {{
        if (closed) return
        closed = true
{child_closes}
        {root_property}.close()
    }}
}}

@Composable
fun remember{container}({remember_args}): {container} {{
    val kit = remember({remember_keys}) {{ {container}({remember_call_args}) }}
    DisposableEffect(kit) {{
        onDispose {{ kit.close() }}
    }}
    return kit
}}
"#,
        package_name = package_name,
        container = graph.container_name,
        ctor_sig = ctor_sig,
        root_property = root_property,
        root_bridge = graph.root.bridge_name,
        root_ctor = ctor_call,
        child_decls = indent_optional(&child_decls, 0),
        child_closes = indent_optional(&child_closes, 0),
        remember_args = root_kotlin_constructor_args(graph.root),
        remember_keys = root_kotlin_remember_keys(graph.root),
        remember_call_args = root_kotlin_constructor_call_args(graph.root),
    )
}

fn validate_swift_bridge_metadata(metadata: &VmMetadata) -> Result<(), CodegenError> {
    metadata.validate()?;
    match metadata.mode {
        VmMode::State | VmMode::DiffList => {
            let observer = metadata
                .observer
                .as_ref()
                .ok_or(MetadataValidationError::MissingField("observer"))?;
            let subscribe = metadata
                .methods
                .iter()
                .find(|method| method.name == "subscribe")
                .ok_or(MetadataValidationError::MissingMethod("subscribe"))?;
            if subscribe.args.len() != 1 || subscribe.args[0].name != "observer" {
                return Err(MetadataValidationError::InvalidMethodShape {
                    method: "subscribe",
                    reason: "must accept exactly one observer argument",
                }
                .into());
            }
            let observer_arg_type = &subscribe.args[0].rust_type;
            let short_observer_type = format!("Arc<dyn {}>", observer.rust_type);
            let full_observer_type = format!("std::sync::Arc<dyn {}>", observer.rust_type);
            if observer_arg_type != &short_observer_type && observer_arg_type != &full_observer_type
            {
                return Err(MetadataValidationError::InvalidMethodShape {
                    method: "subscribe",
                    reason: "observer argument type must match observer",
                }
                .into());
            }
            if let Some(unsubscribe) = metadata.methods.iter().find(|m| m.name == "unsubscribe") {
                if unsubscribe.args.len() != 1 || unsubscribe.args[0].name != "id" {
                    return Err(MetadataValidationError::InvalidMethodShape {
                        method: "unsubscribe",
                        reason: "must accept exactly one id argument",
                    }
                    .into());
                }
                if subscribe.return_type != "unit"
                    && unsubscribe.args[0].rust_type != subscribe.return_type
                {
                    return Err(MetadataValidationError::InvalidMethodShape {
                        method: "unsubscribe",
                        reason: "id argument type must match subscribe return type",
                    }
                    .into());
                }
                if unsubscribe.return_type != "unit" {
                    return Err(MetadataValidationError::InvalidMethodShape {
                        method: "unsubscribe",
                        reason: "must return unit",
                    }
                    .into());
                }
            }
        }
        VmMode::Event | VmMode::Unknown => {}
    }
    Ok(())
}

struct SubscribeInfo {
    ret_type: String,
}

fn generate_state_bridge(metadata: &VmMetadata) -> String {
    let observer = metadata.observer.as_ref().unwrap();
    let state_type = metadata.state_type.as_deref().unwrap();
    let (ctor_sig, ctor_call) = constructor_or_factory(metadata);
    let subscribe_info = subscribe_info(metadata);
    let unsubscribe_name = unsubscribe_method_name(metadata);
    let observer_method = to_swift_method_name(&observer.method);

    let mut methods = String::new();
    for method in filtered_methods(metadata) {
        methods.push_str(&format_swift_method(method));
    }

    let observer_proxy = format!(
        r#"private final class ObserverProxy: {observer} {{
    weak var owner: {bridge}?
    init(owner: {bridge}) {{
        self.owner = owner
    }}
    func {observer_method}(state: {state_type}) {{
        Task {{ @MainActor [weak owner] in
            owner?.{observer_method}(state: state)
        }}
    }}
}}
"#,
        observer = observer.rust_type,
        bridge = metadata.bridge_name,
        observer_method = observer_method,
        state_type = state_type
    );

    let (observer_id_decl, observer_id_assign, observer_id_unsubscribe) = subscribe_info
        .as_ref()
        .map(|info| {
            let decl = format!("private var observerId: {}?", info.ret_type);
            let assign = "self.observerId = vm.subscribe(observer: observer)".to_string();
            let unsubscribe = if let Some(name) = &unsubscribe_name {
                format!(
                    "deinit {{\n        if let id = observerId {{\n            vm.{name}(id: id)\n        }}\n    }}\n",
                    name = name
                )
            } else {
                String::new()
            };
            (decl, assign, unsubscribe)
        })
        .unwrap_or_else(|| {
            (
                String::new(),
                "vm.subscribe(observer: observer)".to_string(),
                String::new(),
            )
        });

    format!(
        r#"// Generated by cross-kit-codegen.
import Combine
import Foundation

{observer_proxy}
@MainActor
public final class {bridge}: ObservableObject, {observer} {{
    @Published public private(set) var state: {state_type}

    private let vm: {vm_type}Protocol
    private var observer: ObserverProxy?
{observer_id_decl}

    {ctor_sig} {{
        let vm = {ctor_call}
        self.vm = vm
        self.state = vm.getState()
        let observer = ObserverProxy(owner: self)
        self.observer = observer
        {observer_id_assign}
    }}

{methods}
    public func {observer_method}(state: {state_type}) {{
        self.state = state
    }}

    {observer_id_unsubscribe}
}}
"#,
        bridge = metadata.bridge_name,
        observer = observer.rust_type,
        state_type = state_type,
        vm_type = metadata.rust_type,
        observer_method = observer_method,
        ctor_sig = ctor_sig,
        ctor_call = ctor_call,
        methods = indent(&methods, 4),
        observer_proxy = observer_proxy.trim_end(),
        observer_id_decl = indent_optional(&observer_id_decl, 4),
        observer_id_assign = observer_id_assign,
        observer_id_unsubscribe =
            indent_optional(&observer_id_unsubscribe.trim_end().to_string(), 4),
    )
}

fn generate_list_bridge(metadata: &VmMetadata) -> String {
    let observer = metadata.observer.as_ref().unwrap();
    let diff_type = metadata.diff_type.as_deref().unwrap();
    let list_item = metadata.list_item_type.as_deref().unwrap();
    let (ctor_sig, ctor_call) = constructor_or_factory(metadata);
    let subscribe_info = subscribe_info(metadata);
    let unsubscribe_name = unsubscribe_method_name(metadata);
    let observer_method = to_swift_method_name(&observer.method);

    let mut methods = String::new();
    for method in filtered_methods(metadata) {
        methods.push_str(&format_swift_method(method));
    }

    let observer_proxy = format!(
        r#"private final class ObserverProxy: {observer} {{
    weak var owner: {bridge}?
    init(owner: {bridge}) {{
        self.owner = owner
    }}
    func {observer_method}(diffs: [{diff_type}]) {{
        Task {{ @MainActor [weak owner] in
            owner?.{observer_method}(diffs: diffs)
        }}
    }}
}}
"#,
        observer = observer.rust_type,
        bridge = metadata.bridge_name,
        observer_method = observer_method,
        diff_type = diff_type
    );

    let (observer_id_decl, observer_id_assign, observer_id_unsubscribe) = subscribe_info
        .as_ref()
        .map(|info| {
            let decl = format!("private var observerId: {}?", info.ret_type);
            let assign = "self.observerId = vm.subscribe(observer: observer)".to_string();
            let unsubscribe = if let Some(name) = &unsubscribe_name {
                format!(
                    "deinit {{\n        if let id = observerId {{\n            vm.{name}(id: id)\n        }}\n    }}\n",
                    name = name
                )
            } else {
                String::new()
            };
            (decl, assign, unsubscribe)
        })
        .unwrap_or_else(|| {
            (
                String::new(),
                "vm.subscribe(observer: observer)".to_string(),
                String::new(),
            )
        });

    format!(
        r#"// Generated by cross-kit-codegen.
import Combine
import Foundation

{observer_proxy}
@MainActor
public final class {bridge}: ObservableObject, {observer} {{
    @Published public private(set) var items: [{list_item}] = []

    private let vm: {vm_type}Protocol
    private var observer: ObserverProxy?
{observer_id_decl}

    {ctor_sig} {{
        let vm = {ctor_call}
        self.vm = vm
        let observer = ObserverProxy(owner: self)
        self.observer = observer
        {observer_id_assign}
    }}

{methods}
    public func {observer_method}(diffs: [{diff_type}]) {{
        for diff in diffs {{
            switch diff {{
            case let .insert(index, item):
                let idx = clampIndex(index, upperBound: items.count)
                items.insert(item, at: idx)
            case let .update(index, item):
                let idx = Int(index)
                guard idx >= 0 && idx < items.count else {{ continue }}
                items[idx] = item
            case let .remove(index, _):
                let idx = Int(index)
                guard idx >= 0 && idx < items.count else {{ continue }}
                items.remove(at: idx)
            case let .move(from, to):
                let fromIdx = Int(from)
                let toIdx = Int(to)
                guard fromIdx >= 0, toIdx >= 0, fromIdx < items.count, toIdx < items.count else {{ continue }}
                if fromIdx == toIdx {{ continue }}
                let item = items.remove(at: fromIdx)
                items.insert(item, at: toIdx)
            }}
        }}
    }}

    private func clampIndex(_ index: Int64, upperBound: Int) -> Int {{
        let idx = Int(index)
        if idx < 0 {{ return 0 }}
        if idx > upperBound {{ return upperBound }}
        return idx
    }}

    {observer_id_unsubscribe}
}}
"#,
        bridge = metadata.bridge_name,
        observer = observer.rust_type,
        list_item = list_item,
        vm_type = metadata.rust_type,
        observer_method = observer_method,
        diff_type = diff_type,
        ctor_sig = ctor_sig,
        ctor_call = ctor_call,
        methods = indent(&methods, 4),
        observer_proxy = observer_proxy.trim_end(),
        observer_id_decl = indent_optional(&observer_id_decl, 4),
        observer_id_assign = observer_id_assign,
        observer_id_unsubscribe =
            indent_optional(&observer_id_unsubscribe.trim_end().to_string(), 4),
    )
}

fn generate_kotlin_state_bridge(metadata: &VmMetadata, package_name: &str) -> String {
    let observer = metadata.observer.as_ref().unwrap();
    let state_type = metadata.state_type.as_deref().unwrap();
    let observer_method = to_kotlin_method_name(&observer.method);
    let (ctor_sig, vm_init) = kotlin_constructor_or_factory(metadata);
    let unsubscribe_name =
        unsubscribe_method_name(metadata).map(|name| to_kotlin_method_name(&name));
    let close_body = unsubscribe_name
        .map(|name| format!("vm.{name}(observerId)\n        vm.close()"))
        .unwrap_or_else(|| "vm.close()".to_string());
    let companion = kotlin_native_companion(metadata);

    let mut methods = String::new();
    for method in filtered_kotlin_methods(metadata) {
        methods.push_str(&format_kotlin_method(method));
    }

    format!(
        r#"// Generated by cross-kit-codegen.
package {package_name}

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

class {bridge}{ctor_sig} : {observer} {{
    private val handler = Handler(Looper.getMainLooper())
    private val vm: {vm_type} = {vm_init}
    private val observerId: Long = vm.subscribe(this)
    private var closed = false

    var state: {state_type} by mutableStateOf(vm.getState())
        private set

{methods}    fun close() {{
        if (closed) return
        closed = true
        {close_body}
    }}

    override fun {observer_method}(state: {state_type}) {{
        if (Looper.myLooper() == Looper.getMainLooper()) {{
            this.state = state
        }} else {{
            handler.post {{ this.state = state }}
        }}
    }}
{companion}
}}
"#,
        package_name = package_name,
        bridge = metadata.bridge_name,
        ctor_sig = ctor_sig,
        observer = observer.rust_type,
        vm_type = metadata.rust_type,
        vm_init = vm_init,
        state_type = state_type,
        methods = indent(&methods, 4),
        close_body = close_body,
        observer_method = observer_method,
        companion = indent_optional(&companion, 4),
    )
}

fn generate_kotlin_list_bridge(metadata: &VmMetadata, package_name: &str) -> String {
    let observer = metadata.observer.as_ref().unwrap();
    let diff_type = metadata.diff_type.as_deref().unwrap();
    let list_item = metadata.list_item_type.as_deref().unwrap();
    let observer_method = to_kotlin_method_name(&observer.method);
    let (ctor_sig, vm_init) = kotlin_constructor_or_factory(metadata);
    let unsubscribe_name =
        unsubscribe_method_name(metadata).map(|name| to_kotlin_method_name(&name));
    let close_body = unsubscribe_name
        .map(|name| format!("vm.{name}(observerId)\n        vm.close()"))
        .unwrap_or_else(|| "vm.close()".to_string());
    let companion = kotlin_native_companion(metadata);

    let mut methods = String::new();
    for method in filtered_kotlin_methods(metadata) {
        methods.push_str(&format_kotlin_method(method));
    }

    format!(
        r#"// Generated by cross-kit-codegen.
package {package_name}

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList

class {bridge}{ctor_sig} : {observer} {{
    private val handler = Handler(Looper.getMainLooper())
    private val vm: {vm_type} = {vm_init}
    private val observerId: Long = vm.subscribe(this)
    private var closed = false

    val items: SnapshotStateList<{list_item}> = mutableStateListOf()

{methods}    fun close() {{
        if (closed) return
        closed = true
        {close_body}
    }}

    override fun {observer_method}(diffs: List<{diff_type}>) {{
        if (Looper.myLooper() == Looper.getMainLooper()) {{
            applyDiffsToItems(diffs)
        }} else {{
            handler.post {{ applyDiffsToItems(diffs) }}
        }}
    }}

    private fun applyDiffsToItems(diffs: List<{diff_type}>) {{
        for (diff in diffs) {{
            when (diff) {{
                is {diff_type}.Insert -> {{
                    val idx = clampIndex(diff.index, items.size)
                    items.add(idx, diff.item)
                }}
                is {diff_type}.Update -> {{
                    val idx = diff.index.toInt()
                    if (idx in items.indices) {{
                        items[idx] = diff.item
                    }}
                }}
                is {diff_type}.Remove -> {{
                    val idx = diff.index.toInt()
                    if (idx in items.indices) {{
                        items.removeAt(idx)
                    }}
                }}
                is {diff_type}.Move -> {{
                    val fromIdx = diff.from.toInt()
                    val toIdx = diff.to.toInt()
                    if (fromIdx !in items.indices || toIdx !in items.indices) continue
                    if (fromIdx == toIdx) continue
                    val item = items.removeAt(fromIdx)
                    items.add(toIdx, item)
                }}
            }}
        }}
    }}

    private fun clampIndex(index: Long, upperBound: Int): Int {{
        val idx = index.toInt()
        return when {{
            idx < 0 -> 0
            idx > upperBound -> upperBound
            else -> idx
        }}
    }}
{companion}
}}
"#,
        package_name = package_name,
        bridge = metadata.bridge_name,
        ctor_sig = ctor_sig,
        observer = observer.rust_type,
        vm_type = metadata.rust_type,
        vm_init = vm_init,
        list_item = list_item,
        methods = indent(&methods, 4),
        close_body = close_body,
        observer_method = observer_method,
        diff_type = diff_type,
        companion = indent_optional(&companion, 4),
    )
}

fn filtered_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| method.name != "subscribe" && method.name != "new")
        .filter(|method| method.name != "unsubscribe")
        .collect()
}

fn filtered_kotlin_methods(metadata: &VmMetadata) -> Vec<&MethodMetadata> {
    metadata
        .methods
        .iter()
        .filter(|method| method.name != "subscribe" && method.name != "new")
        .filter(|method| method.name != "unsubscribe")
        .filter(|method| method.name != "get_state")
        .collect()
}

fn constructor_method(metadata: &VmMetadata) -> Option<&MethodMetadata> {
    metadata.methods.iter().find(|method| method.name == "new")
}

fn constructor_or_factory(metadata: &VmMetadata) -> (String, String) {
    if let Some(factory) = &metadata.factory {
        let method = to_swift_method_name(&factory.method);
        return (
            format!("public init(app: {})", factory.bridge_name),
            format!("app.{method}()"),
        );
    }
    if let Some(ctor) = constructor_method(metadata) {
        return (
            format_swift_signature(ctor, true),
            format_swift_ctor_call(ctor, &metadata.rust_type),
        );
    }
    (
        "public init()".to_string(),
        format!("{}()", metadata.rust_type),
    )
}

fn subscribe_info(metadata: &VmMetadata) -> Option<SubscribeInfo> {
    let method = metadata.methods.iter().find(|m| m.name == "subscribe")?;
    let ret_type = map_type_to_swift(&method.return_type);
    if ret_type == "Void" {
        None
    } else {
        Some(SubscribeInfo { ret_type })
    }
}

fn unsubscribe_method_name(metadata: &VmMetadata) -> Option<String> {
    metadata
        .methods
        .iter()
        .find(|m| m.name == "unsubscribe")
        .map(|m| to_swift_method_name(&m.name))
}

fn format_swift_signature(method: &MethodMetadata, with_public: bool) -> String {
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_swift_method_name(&arg.name),
                map_type_to_swift(&arg.rust_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    if with_public {
        format!("public init({})", args)
    } else {
        format!("init({})", args)
    }
}

fn format_swift_ctor_call(method: &MethodMetadata, vm_type: &str) -> String {
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_swift_method_name(&arg.name),
                to_swift_method_name(&arg.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({})", vm_type, args)
}

fn format_swift_method(method: &MethodMetadata) -> String {
    let swift_name = to_swift_method_name(&method.name);
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_swift_method_name(&arg.name),
                map_type_to_swift(&arg.rust_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = method
        .args
        .iter()
        .map(|arg| {
            let label = to_swift_method_name(&arg.name);
            format!("{label}: {label}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret_type = map_type_to_swift(&method.return_type);
    let ret_sig = if ret_type == "Void" {
        "".to_string()
    } else {
        format!(" -> {ret_type}")
    };
    let call = if ret_type == "Void" {
        format!("vm.{}({})", swift_name, call_args)
    } else {
        format!("return vm.{}({})", swift_name, call_args)
    };
    format!(
        "public func {swift_name}({args}){ret_sig} {{\n        {call}\n    }}\n\n",
        swift_name = swift_name,
        args = args,
        ret_sig = ret_sig,
        call = call
    )
}

fn kotlin_constructor_or_factory(metadata: &VmMetadata) -> (String, String) {
    if metadata.factory.is_some() {
        return (
            format!(" private constructor(vm: {})", metadata.rust_type),
            "vm".to_string(),
        );
    }
    if let Some(ctor) = constructor_method(metadata) {
        return (
            format_kotlin_constructor_signature(ctor),
            format_kotlin_ctor_call(ctor, &metadata.rust_type),
        );
    }
    ("()".to_string(), format!("{}()", metadata.rust_type))
}

fn kotlin_native_companion(metadata: &VmMetadata) -> String {
    if metadata.factory.is_none() {
        return String::new();
    }
    format!(
        r#"

companion object {{
    internal fun __crossKitFromVm(vm: {vm_type}): {bridge} = {bridge}(vm)
}}
"#,
        vm_type = metadata.rust_type,
        bridge = metadata.bridge_name,
    )
}

fn format_kotlin_constructor_signature(method: &MethodMetadata) -> String {
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_kotlin_method_name(&arg.name),
                map_type_to_kotlin(&arg.rust_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("({args})")
}

fn format_kotlin_ctor_call(method: &MethodMetadata, vm_type: &str) -> String {
    let args = method
        .args
        .iter()
        .map(|arg| to_kotlin_method_name(&arg.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({})", vm_type, args)
}

fn root_constructor_for_container(metadata: &VmMetadata) -> (String, String) {
    if let Some(ctor) = constructor_method(metadata) {
        return (
            format_swift_signature(ctor, true),
            format_swift_ctor_call(ctor, &metadata.bridge_name),
        );
    }
    (
        "public init()".to_string(),
        format!("{}()", metadata.bridge_name),
    )
}

fn root_kotlin_constructor_for_container(metadata: &VmMetadata) -> (String, String) {
    if let Some(ctor) = constructor_method(metadata) {
        return (
            format_kotlin_constructor_signature(ctor),
            format_kotlin_ctor_call(ctor, &metadata.bridge_name),
        );
    }
    ("()".to_string(), format!("{}()", metadata.bridge_name))
}

fn root_kotlin_constructor_args(metadata: &VmMetadata) -> String {
    constructor_method(metadata)
        .map(|ctor| {
            ctor.args
                .iter()
                .map(|arg| {
                    format!(
                        "{}: {}",
                        to_kotlin_method_name(&arg.name),
                        map_type_to_kotlin(&arg.rust_type)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn root_kotlin_constructor_call_args(metadata: &VmMetadata) -> String {
    constructor_method(metadata)
        .map(|ctor| {
            ctor.args
                .iter()
                .map(|arg| to_kotlin_method_name(&arg.name))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn root_kotlin_remember_keys(metadata: &VmMetadata) -> String {
    let args = root_kotlin_constructor_call_args(metadata);
    if args.is_empty() {
        "Unit".to_string()
    } else {
        args
    }
}

fn format_kotlin_method(method: &MethodMetadata) -> String {
    let kotlin_name = to_kotlin_method_name(&method.name);
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_kotlin_method_name(&arg.name),
                map_type_to_kotlin(&arg.rust_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = method
        .args
        .iter()
        .map(|arg| to_kotlin_method_name(&arg.name))
        .collect::<Vec<_>>()
        .join(", ");
    let arc_return = owned_arc_return_type(&method.return_type);
    let ret_type = arc_return
        .map(|rust_type| kotlin_bridge_name_for_rust_type(rust_type))
        .unwrap_or_else(|| map_type_to_kotlin(&method.return_type));
    let call = if let Some(rust_type) = arc_return {
        let bridge = kotlin_bridge_name_for_rust_type(rust_type);
        format!("return {bridge}.__crossKitFromVm(vm.{kotlin_name}({call_args}))")
    } else if ret_type == "Unit" {
        format!("vm.{}({})", kotlin_name, call_args)
    } else {
        format!("return vm.{}({})", kotlin_name, call_args)
    };
    format!(
        "fun {kotlin_name}({args}): {ret_type} {{\n        {call}\n    }}\n\n",
        kotlin_name = kotlin_name,
        args = args,
        ret_type = ret_type,
        call = call
    )
}

fn owned_arc_return_type(ty: &str) -> Option<&str> {
    let ty = ty.trim();
    let inner = ty
        .strip_prefix("Arc<")
        .or_else(|| ty.strip_prefix("std::sync::Arc<"))?
        .strip_suffix('>')?
        .trim();
    if inner.starts_with("dyn ") {
        None
    } else {
        Some(inner)
    }
}

fn kotlin_bridge_name_for_rust_type(rust_type: &str) -> String {
    format!("{rust_type}Bridge")
}

fn property_name(rust_type: &str) -> String {
    let mut base = rust_type
        .strip_suffix("ViewModel")
        .unwrap_or(rust_type)
        .to_string();
    if base.is_empty() {
        base = rust_type.to_string();
    }
    let mut chars = base.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_ascii_lowercase(), chars.as_str())
}

fn validate_cross_platform_identifier(kind: &str, value: &str) -> Result<(), CodegenError> {
    if !is_ascii_identifier(value) || is_swift_or_kotlin_keyword(value) {
        return Err(CodegenError::InvalidRootGraph(format!(
            "{kind} must be a valid Swift/Kotlin identifier, got '{value}'"
        )));
    }
    Ok(())
}

fn is_reserved_root_container_property(value: &str) -> bool {
    matches!(
        value,
        "cancellables" | "closed" | "close" | "objectWillChange"
    )
}

fn generated_platform_type_names(metadata: &VmMetadata) -> Vec<String> {
    let mut names = vec![
        metadata.rust_type.clone(),
        metadata.bridge_name.clone(),
        format!("{}Protocol", metadata.rust_type),
    ];
    if let Some(observer) = &metadata.observer {
        names.push(observer.rust_type.clone());
        names.push(format!("{}Protocol", observer.rust_type));
    }
    for ty in [
        metadata.state_type.as_ref(),
        metadata.diff_type.as_ref(),
        metadata.list_item_type.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        names.push(ty.clone());
    }
    names
}

fn is_ascii_identifier(value: &str) -> bool {
    if value == "_" {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_swift_or_kotlin_keyword(value: &str) -> bool {
    matches!(
        value,
        "Any"
            | "Self"
            | "actor"
            | "actual"
            | "abstract"
            | "annotation"
            | "as"
            | "associatedtype"
            | "async"
            | "await"
            | "break"
            | "by"
            | "case"
            | "catch"
            | "class"
            | "companion"
            | "const"
            | "constructor"
            | "continue"
            | "convenience"
            | "data"
            | "default"
            | "defer"
            | "deinit"
            | "do"
            | "dynamic"
            | "else"
            | "enum"
            | "expect"
            | "extension"
            | "external"
            | "fallthrough"
            | "false"
            | "fileprivate"
            | "final"
            | "finally"
            | "for"
            | "fun"
            | "func"
            | "if"
            | "import"
            | "in"
            | "indirect"
            | "infix"
            | "init"
            | "inner"
            | "inout"
            | "interface"
            | "internal"
            | "is"
            | "lateinit"
            | "let"
            | "mutating"
            | "nil"
            | "noinline"
            | "nonmutating"
            | "null"
            | "object"
            | "open"
            | "operator"
            | "out"
            | "override"
            | "package"
            | "precedencegroup"
            | "private"
            | "protected"
            | "protocol"
            | "public"
            | "reified"
            | "required"
            | "return"
            | "rethrows"
            | "sealed"
            | "self"
            | "static"
            | "struct"
            | "subscript"
            | "super"
            | "suspend"
            | "switch"
            | "this"
            | "throw"
            | "throws"
            | "true"
            | "try"
            | "typealias"
            | "typeof"
            | "unowned"
            | "val"
            | "var"
            | "vararg"
            | "weak"
            | "when"
            | "where"
            | "while"
    )
}

fn map_type_to_kotlin(ty: &str) -> String {
    let ty = ty.trim();
    let ty = if ty == "crate::runtime::SubscriptionId" {
        "SubscriptionId"
    } else {
        ty
    };
    if ty == "unit" {
        return "Unit".to_string();
    }
    if let Some(inner) = ty.strip_prefix("Arc<").and_then(|v| v.strip_suffix('>')) {
        return map_type_to_kotlin(inner.trim_start_matches("dyn "));
    }
    if let Some(inner) = ty
        .strip_prefix("std::sync::Arc<")
        .and_then(|v| v.strip_suffix('>'))
    {
        return map_type_to_kotlin(inner.trim_start_matches("dyn "));
    }
    if let Some(inner) = ty.strip_prefix("dyn ") {
        return map_type_to_kotlin(inner);
    }
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|v| v.strip_suffix('>')) {
        return format!("{}?", map_type_to_kotlin(inner));
    }
    if let Some(inner) = ty.strip_prefix("Vec<").and_then(|v| v.strip_suffix('>')) {
        return format!("List<{}>", map_type_to_kotlin(inner));
    }
    match ty {
        "i64" | "SubscriptionId" | "cross_kit::SubscriptionId" | "crate::SubscriptionId" => {
            "Long".to_string()
        }
        "i32" => "Int".to_string(),
        "u64" => "ULong".to_string(),
        "u32" => "UInt".to_string(),
        "bool" => "Boolean".to_string(),
        "String" => "String".to_string(),
        other => other.to_string(),
    }
}

fn to_kotlin_method_name(name: &str) -> String {
    to_swift_method_name(name)
}

fn map_type_to_swift(ty: &str) -> String {
    let ty = ty.trim();
    let ty = if ty == "crate::runtime::SubscriptionId" {
        "SubscriptionId"
    } else {
        ty
    };
    if ty == "unit" {
        return "Void".to_string();
    }
    if let Some(inner) = ty.strip_prefix("Arc<").and_then(|v| v.strip_suffix('>')) {
        let inner = map_type_to_swift(inner);
        if ty.contains("Arc<dyn ") {
            return inner;
        }
        return format!("{inner}Protocol");
    }
    if let Some(inner) = ty
        .strip_prefix("std::sync::Arc<")
        .and_then(|v| v.strip_suffix('>'))
    {
        let inner = map_type_to_swift(inner);
        if ty.contains("Arc<dyn ") {
            return inner;
        }
        return format!("{inner}Protocol");
    }
    if let Some(inner) = ty.strip_prefix("dyn ") {
        return map_type_to_swift(inner);
    }
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|v| v.strip_suffix('>')) {
        return format!("{}?", map_type_to_swift(inner));
    }
    if let Some(inner) = ty.strip_prefix("Vec<").and_then(|v| v.strip_suffix('>')) {
        return format!("[{}]", map_type_to_swift(inner));
    }
    match ty {
        "i64" | "SubscriptionId" | "cross_kit::SubscriptionId" | "crate::SubscriptionId" => {
            "Int64".to_string()
        }
        "i32" => "Int32".to_string(),
        "u64" => "UInt64".to_string(),
        "u32" => "UInt32".to_string(),
        "bool" => "Bool".to_string(),
        "String" => "String".to_string(),
        other => other.to_string(),
    }
}

fn to_swift_method_name(name: &str) -> String {
    let mut parts = name.split('_');
    let first = parts.next().unwrap_or("").to_string();
    let mut result = first;
    for part in parts {
        let mut chars = part.chars();
        if let Some(first_char) = chars.next() {
            result.push(first_char.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
    }
    result
}

fn indent(input: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    input
        .lines()
        .map(|line| {
            if line.is_empty() {
                line.to_string()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent_optional(input: &str, spaces: usize) -> String {
    if input.trim().is_empty() {
        String::new()
    } else {
        indent(input, spaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cross_kit_core::{
        ArgMetadata, BindingsConfig, FactoryMetadata, MethodMetadata, ObserverMetadata,
        VM_METADATA_SCHEMA_VERSION,
    };

    fn state_metadata() -> VmMetadata {
        VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: Some(FactoryMetadata {
                rust_type: "AppViewModel".to_string(),
                method: "make_counter_vm".to_string(),
                bridge_name: "AppViewModelBridge".to_string(),
            }),
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "unsubscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "id".to_string(),
                        rust_type: "i64".to_string(),
                    }],
                    return_type: "unit".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "CounterState".to_string(),
                },
                MethodMetadata {
                    name: "increment_by".to_string(),
                    args: vec![ArgMetadata {
                        name: "delta_value".to_string(),
                        rust_type: "i32".to_string(),
                    }],
                    return_type: "CounterState".to_string(),
                },
            ],
        }
    }

    fn list_metadata() -> VmMetadata {
        VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn ListObserver>".to_string(),
                    }],
                    return_type: "unit".to_string(),
                },
                MethodMetadata {
                    name: "append_now".to_string(),
                    args: Vec::new(),
                    return_type: "ListItem".to_string(),
                },
                MethodMetadata {
                    name: "apply_diffs".to_string(),
                    args: vec![ArgMetadata {
                        name: "diffs".to_string(),
                        rust_type: "Vec<ListDiff>".to_string(),
                    }],
                    return_type: "bool".to_string(),
                },
            ],
        }
    }

    fn app_metadata() -> VmMetadata {
        let mut metadata = state_metadata();
        metadata.rust_type = "AppViewModel".to_string();
        metadata.bridge_name = "AppViewModelBridge".to_string();
        metadata.factory = None;
        metadata.observer.as_mut().unwrap().rust_type = "AppObserver".to_string();
        metadata.methods[0].args[0].rust_type = "Arc<dyn AppObserver>".to_string();
        metadata.state_type = Some("AppState".to_string());
        metadata.methods[2].return_type = "AppState".to_string();
        metadata.methods.push(MethodMetadata {
            name: "new".to_string(),
            args: vec![ArgMetadata {
                name: "initial".to_string(),
                rust_type: "i32".to_string(),
            }],
            return_type: "Arc<Self>".to_string(),
        });
        metadata.methods.push(MethodMetadata {
            name: "clear_route".to_string(),
            args: Vec::new(),
            return_type: "unit".to_string(),
        });
        metadata.methods.push(MethodMetadata {
            name: "make_counter_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<CounterViewModel>".to_string(),
        });
        metadata.methods.push(MethodMetadata {
            name: "make_list_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<ListViewModel>".to_string(),
        });
        metadata
    }

    fn child_list_metadata() -> VmMetadata {
        let mut metadata = list_metadata();
        metadata.factory = Some(FactoryMetadata {
            rust_type: "AppViewModel".to_string(),
            method: "make_list_vm".to_string(),
            bridge_name: "AppViewModelBridge".to_string(),
        });
        metadata
    }

    fn bindings() -> BindingsConfig {
        BindingsConfig {
            root_vm: "AppViewModel".to_string(),
            container_name: "CrossKitSharedBridge".to_string(),
        }
    }

    #[test]
    fn generates_state_bridge_with_factory_and_unsubscribe() {
        let files = generate_swift_bridge(&state_metadata()).unwrap();
        let code = &files.files[0].contents;

        assert_eq!(files.files[0].path, "CounterViewModelBridge.swift");
        assert!(code.contains("// Generated by cross-kit-codegen."));
        assert!(code.contains("public final class CounterViewModelBridge"));
        assert!(code.contains("public init(app: AppViewModelBridge)"));
        assert!(code.contains("let vm = app.makeCounterVm()"));
        assert!(code.contains("private var observerId: Int64?"));
        assert!(code.contains("vm.unsubscribe(id: id)"));
        assert!(code.contains("public func incrementBy(deltaValue: Int32) -> CounterState"));
        assert!(!code.contains("public func subscribe"));
    }

    #[test]
    fn generates_diff_list_bridge_with_diff_application() {
        let code = generate_swift_bridge_source(&list_metadata()).unwrap();

        assert!(code.contains("@Published public private(set) var items: [ListItem] = []"));
        assert!(code.contains("public func appendNow() -> ListItem"));
        assert!(code.contains("public func applyDiffs(diffs: [ListDiff]) -> Bool"));
        assert!(code.contains("case let .move(from, to):"));
        assert!(code.contains("items.insert(item, at: toIdx)"));
        assert!(!code.contains("fromIdx < toIdx ? toIdx - 1 : toIdx"));
        assert!(code.contains("private func clampIndex"));
        assert!(code.contains("vm.subscribe(observer: observer)"));
    }

    #[test]
    fn generates_kotlin_state_bridge_with_main_thread_dispatch_and_close() {
        let files = generate_kotlin_bridge(&state_metadata(), "com.crosskit.shared").unwrap();
        let code = &files.files[0].contents;

        assert_eq!(
            files.files[0].path,
            "com/crosskit/shared/CounterViewModelBridge.kt"
        );
        assert!(code.contains("package com.crosskit.shared"));
        assert!(
            code.contains("class CounterViewModelBridge private constructor(vm: CounterViewModel)")
        );
        assert!(code.contains("private val handler = Handler(Looper.getMainLooper())"));
        assert!(code.contains("var state: CounterState by mutableStateOf(vm.getState())"));
        assert!(code.contains("private val observerId: Long = vm.subscribe(this)"));
        assert!(code.contains("fun incrementBy(deltaValue: Int): CounterState"));
        assert!(code.contains("fun close()"));
        assert!(code.contains("private var closed = false"));
        assert!(code.contains("if (closed) return"));
        assert!(code.contains("vm.unsubscribe(observerId)"));
        assert!(code.contains("vm.close()"));
        assert!(code.contains("internal fun __crossKitFromVm(vm: CounterViewModel)"));
        assert!(code.contains("handler.post { this.state = state }"));
        assert!(!code.contains("__crossKitVm"));
        assert!(!code.contains("System.loadLibrary"));
    }

    #[test]
    fn generated_swift_bridge_maps_subscription_id_alias() {
        let mut metadata = state_metadata();
        metadata.methods[0].return_type = "SubscriptionId".to_string();
        metadata.methods[1].args[0].rust_type = "SubscriptionId".to_string();

        let code = generate_swift_bridge_source(&metadata).unwrap();

        assert!(code.contains("private var observerId: Int64?"));
        assert!(code.contains("self.observerId = vm.subscribe(observer: observer)"));
        assert!(code.contains("vm.unsubscribe(id: id)"));
        assert!(!code.contains("SubscriptionId"));
        assert!(!code.contains("cross_kit::SubscriptionId"));
    }

    #[test]
    fn generated_kotlin_bridge_maps_subscription_id_alias() {
        let mut metadata = state_metadata();
        metadata.methods[0].return_type = "crate::runtime::SubscriptionId".to_string();
        metadata.methods[1].args[0].rust_type = "crate::runtime::SubscriptionId".to_string();

        let code = generate_kotlin_bridge_source(&metadata, "com.crosskit.shared").unwrap();

        assert!(code.contains("private val observerId: Long = vm.subscribe(this)"));
        assert!(code.contains("vm.unsubscribe(observerId)"));
        assert!(!code.contains("SubscriptionId"));
        assert!(!code.contains("cross_kit::SubscriptionId"));
        assert!(!code.contains("crate::runtime::SubscriptionId"));
    }

    #[test]
    fn generates_kotlin_root_state_bridge_with_constructor_args_and_unit_methods() {
        let mut metadata = state_metadata();
        metadata.rust_type = "AppViewModel".to_string();
        metadata.bridge_name = "AppViewModelBridge".to_string();
        metadata.factory = None;
        metadata.observer.as_mut().unwrap().rust_type = "AppObserver".to_string();
        metadata.methods[0].args[0].rust_type = "Arc<dyn AppObserver>".to_string();
        metadata.state_type = Some("AppState".to_string());
        metadata.methods.push(MethodMetadata {
            name: "new".to_string(),
            args: vec![ArgMetadata {
                name: "initial".to_string(),
                rust_type: "i32".to_string(),
            }],
            return_type: "Arc<Self>".to_string(),
        });
        metadata.methods.push(MethodMetadata {
            name: "clear_route".to_string(),
            args: Vec::new(),
            return_type: "unit".to_string(),
        });
        metadata.methods.push(MethodMetadata {
            name: "make_counter_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<CounterViewModel>".to_string(),
        });
        metadata.methods[2].return_type = "AppState".to_string();

        let code = generate_kotlin_bridge_source(&metadata, "com.crosskit.shared").unwrap();

        assert!(code.contains("class AppViewModelBridge(initial: Int)"));
        assert!(code.contains("private val vm: AppViewModel = AppViewModel(initial)"));
        assert!(code.contains("fun clearRoute(): Unit"));
        assert!(code.contains("vm.clearRoute()"));
        assert!(code.contains("fun makeCounterVm(): CounterViewModelBridge"));
        assert!(
            code.contains("return CounterViewModelBridge.__crossKitFromVm(vm.makeCounterVm())")
        );
        assert!(!code.contains("__crossKitVm"));
    }

    #[test]
    fn generates_kotlin_diff_list_bridge_with_invalid_diff_guards() {
        let code = generate_kotlin_bridge_source(&list_metadata(), "com.crosskit.shared").unwrap();

        assert!(code.contains("val items: SnapshotStateList<ListItem> = mutableStateListOf()"));
        assert!(code.contains("fun appendNow(): ListItem"));
        assert!(code.contains("fun applyDiffs(diffs: List<ListDiff>): Boolean"));
        assert!(code.contains("is ListDiff.Move ->"));
        assert!(
            code.contains("if (fromIdx !in items.indices || toIdx !in items.indices) continue")
        );
        assert!(code.contains("items.add(toIdx, item)"));
        assert!(!code.contains("if (fromIdx < toIdx) toIdx - 1 else toIdx"));
        assert!(code.contains("private fun clampIndex(index: Long, upperBound: Int): Int"));
        assert!(code.contains("handler.post { applyDiffsToItems(diffs) }"));
        assert!(code.contains("vm.close()"));
    }

    #[test]
    fn generates_swift_root_container_with_forwarding_and_stable_children() {
        let files = generate_swift_root_container(
            &[app_metadata(), state_metadata(), child_list_metadata()],
            &bindings(),
        )
        .unwrap();
        let code = &files.files[0].contents;

        assert_eq!(files.files[0].path, "CrossKitSharedBridge.swift");
        assert!(code.contains("import Combine"));
        assert!(code.contains("@MainActor"));
        assert!(code.contains("public final class CrossKitSharedBridge: ObservableObject"));
        assert!(code.contains("public let app: AppViewModelBridge"));
        assert!(code.contains("public let counter: CounterViewModelBridge"));
        assert!(code.contains("public let list: ListViewModelBridge"));
        assert!(code.contains("private var cancellables: Set<AnyCancellable> = []"));
        assert!(code.contains("public init(initial: Int32)"));
        assert!(code.contains("let app = AppViewModelBridge(initial: initial)"));
        assert!(code.contains("self.counter = CounterViewModelBridge(app: app)"));
        assert!(code.contains("self.list = ListViewModelBridge(app: app)"));
        assert!(
            code.find("self.counter = CounterViewModelBridge(app: app)")
                .unwrap()
                < code
                    .find("self.list = ListViewModelBridge(app: app)")
                    .unwrap()
        );
        assert!(code.contains(
            "app.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }"
        ));
        assert!(code.contains(
            "counter.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }"
        ));
        assert!(code.contains(
            "list.objectWillChange.sink { [weak self] _ in self?.objectWillChange.send() }"
        ));
    }

    #[test]
    fn generates_kotlin_root_container_with_remember_helper_and_idempotent_close() {
        let files = generate_kotlin_root_container(
            &[app_metadata(), state_metadata(), child_list_metadata()],
            &bindings(),
            "com.crosskit.shared",
        )
        .unwrap();
        let code = &files.files[0].contents;

        assert_eq!(
            files.files[0].path,
            "com/crosskit/shared/CrossKitSharedBridge.kt"
        );
        assert!(code.contains("package com.crosskit.shared"));
        assert!(code.contains("class CrossKitSharedBridge(initial: Int) : AutoCloseable"));
        assert!(code.contains("val app: AppViewModelBridge = AppViewModelBridge(initial)"));
        assert!(code.contains("val counter: CounterViewModelBridge = app.makeCounterVm()"));
        assert!(code.contains("val list: ListViewModelBridge = app.makeListVm()"));
        assert!(code.contains("private var closed = false"));
        assert!(code.contains("if (closed) return"));
        assert!(code.find("list.close()").unwrap() < code.find("counter.close()").unwrap());
        assert!(code.find("counter.close()").unwrap() < code.find("app.close()").unwrap());
        assert!(code.contains("@Composable"));
        assert!(
            code.contains("fun rememberCrossKitSharedBridge(initial: Int): CrossKitSharedBridge")
        );
        assert!(code.contains("val kit = remember(initial) { CrossKitSharedBridge(initial) }"));
        assert!(code.contains("DisposableEffect(kit)"));
        assert!(code.contains("onDispose { kit.close() }"));
    }

    #[test]
    fn generates_root_container_without_children() {
        let swift = generate_swift_root_container(&[app_metadata()], &bindings())
            .unwrap()
            .files
            .remove(0)
            .contents;
        let kotlin =
            generate_kotlin_root_container(&[app_metadata()], &bindings(), "com.crosskit.shared")
                .unwrap()
                .files
                .remove(0)
                .contents;

        assert!(swift.contains("public let app: AppViewModelBridge"));
        assert!(!swift.contains("public let counter"));
        assert!(!kotlin.contains("counter.close()"));
        assert!(kotlin.contains("app.close()"));
    }

    #[test]
    fn generates_zero_arg_root_container_without_children() {
        let mut root = app_metadata();
        root.methods.retain(|method| method.name != "new");
        root.methods
            .retain(|method| !method.name.starts_with("make_"));
        let swift = generate_swift_root_container(&[root.clone()], &bindings())
            .unwrap()
            .files
            .remove(0)
            .contents;
        let kotlin = generate_kotlin_root_container(&[root], &bindings(), "com.crosskit.shared")
            .unwrap()
            .files
            .remove(0)
            .contents;

        assert!(swift.contains("public init()"));
        assert!(swift.contains("let app = AppViewModelBridge()"));
        assert!(kotlin.contains("class CrossKitSharedBridge() : AutoCloseable"));
        assert!(kotlin.contains("val app: AppViewModelBridge = AppViewModelBridge()"));
        assert!(kotlin.contains("fun rememberCrossKitSharedBridge(): CrossKitSharedBridge"));
        assert!(kotlin.contains("val kit = remember(Unit) { CrossKitSharedBridge() }"));
    }

    #[test]
    fn rejects_root_graph_with_missing_bindings_or_metadata() {
        let mut empty_root = bindings();
        empty_root.root_vm.clear();
        let err = generate_swift_root_container(&[app_metadata()], &empty_root).unwrap_err();
        assert!(err.to_string().contains("[bindings].root_vm"));

        let mut empty_container = bindings();
        empty_container.container_name.clear();
        let err = generate_swift_root_container(&[app_metadata()], &empty_container).unwrap_err();
        assert!(err.to_string().contains("[bindings].container_name"));

        let err = generate_swift_root_container(&[], &bindings()).unwrap_err();
        assert!(err.to_string().contains("requires at least one VM"));

        let err = generate_swift_root_container(&[app_metadata(), app_metadata()], &bindings())
            .unwrap_err();
        assert!(err.to_string().contains("is ambiguous in metadata"));

        let mut orphan = state_metadata();
        orphan.factory = None;
        let err =
            generate_swift_root_container(&[app_metadata(), orphan], &bindings()).unwrap_err();
        assert!(err.to_string().contains("must be created by root VM"));
    }

    #[test]
    fn rejects_invalid_root_graph_contracts() {
        let err = generate_swift_root_container(&[state_metadata()], &bindings()).unwrap_err();
        assert!(
            err.to_string()
                .contains("root VM 'AppViewModel' was not found")
        );

        let mut root_with_factory = app_metadata();
        root_with_factory.factory = Some(FactoryMetadata {
            rust_type: "OtherRoot".to_string(),
            method: "make_app_vm".to_string(),
            bridge_name: "OtherRootBridge".to_string(),
        });
        let err = generate_swift_root_container(&[root_with_factory], &bindings()).unwrap_err();
        assert!(err.to_string().contains("must not be created by a factory"));

        let mut wrong_parent = state_metadata();
        wrong_parent.factory.as_mut().unwrap().rust_type = "OtherRoot".to_string();
        let err =
            generate_kotlin_root_container(&[app_metadata(), wrong_parent], &bindings(), "pkg")
                .unwrap_err();
        assert!(err.to_string().contains("factory points to 'OtherRoot'"));

        let mut wrong_factory_bridge = state_metadata();
        wrong_factory_bridge.factory.as_mut().unwrap().bridge_name = "OtherRootBridge".to_string();
        let err =
            generate_swift_root_container(&[app_metadata(), wrong_factory_bridge], &bindings())
                .unwrap_err();
        assert!(err.to_string().contains("does not match root bridge"));

        let mut missing_method = state_metadata();
        missing_method.factory.as_mut().unwrap().method = "make_missing_vm".to_string();
        let err = generate_swift_root_container(&[app_metadata(), missing_method], &bindings())
            .unwrap_err();
        assert!(err.to_string().contains("missing child factory method"));

        let mut arg_factory_root = app_metadata();
        arg_factory_root
            .methods
            .iter_mut()
            .find(|method| method.name == "make_counter_vm")
            .unwrap()
            .args
            .push(ArgMetadata {
                name: "id".to_string(),
                rust_type: "i64".to_string(),
            });
        let err = generate_swift_root_container(&[arg_factory_root, state_metadata()], &bindings())
            .unwrap_err();
        assert!(err.to_string().contains("must not accept arguments"));

        let mut wrong_return_root = app_metadata();
        wrong_return_root
            .methods
            .iter_mut()
            .find(|method| method.name == "make_counter_vm")
            .unwrap()
            .return_type = "Arc<WrongViewModel>".to_string();
        let err =
            generate_swift_root_container(&[wrong_return_root, state_metadata()], &bindings())
                .unwrap_err();
        assert!(
            err.to_string()
                .contains("must return Arc<CounterViewModel>")
        );

        let mut duplicate_bridge = child_list_metadata();
        duplicate_bridge.bridge_name = "CounterViewModelBridge".to_string();
        let err = generate_swift_root_container(
            &[app_metadata(), state_metadata(), duplicate_bridge],
            &bindings(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("used by more than one VM"));

        for bad_container in ["123Bridge", "Cross-Kit", "class", "Self", "_"] {
            let mut bindings = bindings();
            bindings.container_name = bad_container.to_string();
            let err = generate_swift_root_container(&[app_metadata()], &bindings).unwrap_err();
            assert!(
                err.to_string()
                    .contains("[bindings].container_name must be a valid Swift/Kotlin identifier")
            );
        }

        let mut conflicting_bindings = bindings();
        conflicting_bindings.container_name = "AppViewModelBridge".to_string();
        let err =
            generate_swift_root_container(&[app_metadata()], &conflicting_bindings).unwrap_err();
        assert!(
            err.to_string()
                .contains("conflicts with generated bridge 'AppViewModelBridge'")
        );

        for generated_type in [
            "AppViewModel",
            "AppState",
            "AppObserver",
            "AppViewModelProtocol",
        ] {
            let mut bindings = bindings();
            bindings.container_name = generated_type.to_string();
            let err = generate_swift_root_container(&[app_metadata()], &bindings).unwrap_err();
            assert!(
                err.to_string()
                    .contains(&format!("conflicts with generated type '{generated_type}'"))
            );
        }

        let mut keyword_property = state_metadata();
        keyword_property.rust_type = "ClassViewModel".to_string();
        keyword_property.bridge_name = "ClassViewModelBridge".to_string();
        keyword_property.factory.as_mut().unwrap().method = "make_class_vm".to_string();
        let mut keyword_root = app_metadata();
        keyword_root.methods.push(MethodMetadata {
            name: "make_class_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<ClassViewModel>".to_string(),
        });
        let err = generate_swift_root_container(&[keyword_root, keyword_property], &bindings())
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("root container property for VM 'ClassViewModel'")
        );

        let mut swift_keyword_root = app_metadata();
        swift_keyword_root.methods.push(MethodMetadata {
            name: "make_inout_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<InoutViewModel>".to_string(),
        });
        swift_keyword_root.methods.push(MethodMetadata {
            name: "make_precedencegroup_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<PrecedencegroupViewModel>".to_string(),
        });
        for (rust_type, bridge_name, factory_method) in [
            ("InoutViewModel", "InoutViewModelBridge", "make_inout_vm"),
            (
                "PrecedencegroupViewModel",
                "PrecedencegroupViewModelBridge",
                "make_precedencegroup_vm",
            ),
        ] {
            let mut keyword_child = state_metadata();
            keyword_child.rust_type = rust_type.to_string();
            keyword_child.bridge_name = bridge_name.to_string();
            keyword_child.factory.as_mut().unwrap().method = factory_method.to_string();
            let err = generate_swift_root_container(
                &[swift_keyword_root.clone(), keyword_child],
                &bindings(),
            )
            .unwrap_err();
            assert!(
                err.to_string()
                    .contains(&format!("root container property for VM '{rust_type}'"))
            );
        }

        let mut collision_root = app_metadata();
        collision_root.methods.push(MethodMetadata {
            name: "make_foo_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<FooViewModel>".to_string(),
        });
        collision_root.methods.push(MethodMetadata {
            name: "make_foo".to_string(),
            args: Vec::new(),
            return_type: "Arc<Foo>".to_string(),
        });
        let mut foo_vm = state_metadata();
        foo_vm.rust_type = "FooViewModel".to_string();
        foo_vm.bridge_name = "FooViewModelBridge".to_string();
        foo_vm.factory.as_mut().unwrap().method = "make_foo_vm".to_string();
        let mut foo = state_metadata();
        foo.rust_type = "Foo".to_string();
        foo.bridge_name = "FooBridge".to_string();
        foo.factory.as_mut().unwrap().method = "make_foo".to_string();
        let err =
            generate_swift_root_container(&[collision_root, foo_vm, foo], &bindings()).unwrap_err();
        assert!(
            err.to_string()
                .contains("both generate root container property 'foo'")
        );

        let mut reserved_root = app_metadata();
        reserved_root.methods.push(MethodMetadata {
            name: "make_cancellables_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<CancellablesViewModel>".to_string(),
        });
        reserved_root.methods.push(MethodMetadata {
            name: "make_closed_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<ClosedViewModel>".to_string(),
        });
        reserved_root.methods.push(MethodMetadata {
            name: "make_close_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<CloseViewModel>".to_string(),
        });
        reserved_root.methods.push(MethodMetadata {
            name: "make_object_will_change_vm".to_string(),
            args: Vec::new(),
            return_type: "Arc<ObjectWillChangeViewModel>".to_string(),
        });
        for (rust_type, bridge_name, factory_method, property) in [
            (
                "CancellablesViewModel",
                "CancellablesViewModelBridge",
                "make_cancellables_vm",
                "cancellables",
            ),
            (
                "ClosedViewModel",
                "ClosedViewModelBridge",
                "make_closed_vm",
                "closed",
            ),
            (
                "CloseViewModel",
                "CloseViewModelBridge",
                "make_close_vm",
                "close",
            ),
            (
                "ObjectWillChangeViewModel",
                "ObjectWillChangeViewModelBridge",
                "make_object_will_change_vm",
                "objectWillChange",
            ),
        ] {
            let mut reserved_child = state_metadata();
            reserved_child.rust_type = rust_type.to_string();
            reserved_child.bridge_name = bridge_name.to_string();
            reserved_child.factory.as_mut().unwrap().method = factory_method.to_string();
            let err = generate_swift_root_container(
                &[reserved_root.clone(), reserved_child],
                &bindings(),
            )
            .unwrap_err();
            assert!(err.to_string().contains(&format!("property '{property}'")));
            assert!(err.to_string().contains("is reserved by generated code"));
        }
    }

    #[test]
    fn rejects_unsupported_event_bridge_for_swift() {
        let mut metadata = list_metadata();
        metadata.mode = VmMode::Event;
        metadata.diff_type = None;
        metadata.list_item_type = None;
        assert_eq!(
            generate_swift_bridge_source(&metadata),
            Err(CodegenError::UnsupportedMode(VmMode::Event))
        );
        assert_eq!(
            generate_kotlin_bridge_source(&metadata, "com.crosskit.shared"),
            Err(CodegenError::UnsupportedMode(VmMode::Event))
        );
    }

    #[test]
    fn maps_rust_types_to_swift_types() {
        assert_eq!(map_type_to_swift("unit"), "Void");
        assert_eq!(map_type_to_swift("i64"), "Int64");
        assert_eq!(map_type_to_swift("SubscriptionId"), "Int64");
        assert_eq!(map_type_to_swift("cross_kit::SubscriptionId"), "Int64");
        assert_eq!(map_type_to_swift("crate::SubscriptionId"), "Int64");
        assert_eq!(map_type_to_swift("crate::runtime::SubscriptionId"), "Int64");
        assert_eq!(
            map_type_to_swift("other::runtime::SubscriptionId"),
            "other::runtime::SubscriptionId"
        );
        assert_eq!(map_type_to_swift("i32"), "Int32");
        assert_eq!(map_type_to_swift("u64"), "UInt64");
        assert_eq!(map_type_to_swift("u32"), "UInt32");
        assert_eq!(map_type_to_swift("bool"), "Bool");
        assert_eq!(map_type_to_swift("String"), "String");
        assert_eq!(map_type_to_swift("Option<Vec<i64>>"), "[Int64]?");
        assert_eq!(
            map_type_to_swift("Arc<CounterViewModel>"),
            "CounterViewModelProtocol"
        );
        assert_eq!(
            map_type_to_swift("Arc<AppViewModel>"),
            "AppViewModelProtocol"
        );
        assert_eq!(
            map_type_to_swift("std::sync::Arc<ListViewModel>"),
            "ListViewModelProtocol"
        );
        assert_eq!(
            map_type_to_swift("Arc<dyn CounterObserver>"),
            "CounterObserver"
        );
        assert_eq!(map_type_to_swift("CustomRecord"), "CustomRecord");
    }

    #[test]
    fn formats_method_names_and_indentation() {
        assert_eq!(to_swift_method_name("get_state"), "getState");
        assert_eq!(to_swift_method_name("append_now"), "appendNow");
        assert_eq!(to_swift_method_name("already"), "already");
        assert_eq!(to_swift_method_name(""), "");
        assert_eq!(
            to_kotlin_method_name("sort_by_timestamp_desc"),
            "sortByTimestampDesc"
        );
        assert_eq!(map_type_to_kotlin("Option<Vec<i64>>"), "List<Long>?");
        assert_eq!(map_type_to_kotlin("SubscriptionId"), "Long");
        assert_eq!(map_type_to_kotlin("cross_kit::SubscriptionId"), "Long");
        assert_eq!(map_type_to_kotlin("crate::SubscriptionId"), "Long");
        assert_eq!(map_type_to_kotlin("crate::runtime::SubscriptionId"), "Long");
        assert_eq!(
            map_type_to_kotlin("other::runtime::SubscriptionId"),
            "other::runtime::SubscriptionId"
        );
        assert_eq!(
            map_type_to_kotlin("Arc<dyn CounterObserver>"),
            "CounterObserver"
        );
        assert_eq!(
            map_type_to_kotlin("std::sync::Arc<dyn CounterObserver>"),
            "CounterObserver"
        );
        assert_eq!(map_type_to_kotlin("dyn CounterObserver"), "CounterObserver");
        assert_eq!(
            owned_arc_return_type("Arc<CounterViewModel>"),
            Some("CounterViewModel")
        );
        assert_eq!(
            owned_arc_return_type("std::sync::Arc<ListViewModel>"),
            Some("ListViewModel")
        );
        assert_eq!(owned_arc_return_type("Arc<dyn CounterObserver>"), None);
        assert_eq!(
            kotlin_bridge_name_for_rust_type("CounterViewModel"),
            "CounterViewModelBridge"
        );
        assert_eq!(map_type_to_kotlin("u64"), "ULong");
        assert_eq!(map_type_to_kotlin("u32"), "UInt");
        assert_eq!(indent("a\n\nb", 2), "  a\n\n  b");
        assert_eq!(indent_optional("  ", 4), "");
        assert_eq!(indent_optional("x", 4), "    x");
        assert_eq!(property_name("ViewModel"), "viewModel");
        assert_eq!(property_name(""), "");
        assert!(!is_ascii_identifier(""));
    }

    #[test]
    fn fixture_metadata_generates_expected_bridge_count() {
        let metadatas: Vec<VmMetadata> =
            serde_json::from_str(include_str!("../../../fixtures/metadata/counter-list.json"))
                .unwrap();
        let files = metadatas
            .iter()
            .map(generate_swift_bridge)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(files.len(), 3);
        assert!(files[0].files[0].contents.contains("AppViewModelBridge"));
        assert!(files[2].files[0].contents.contains("ListViewModelBridge"));
    }

    #[test]
    fn state_bridge_handles_void_subscription_without_unsubscribe() {
        let mut metadata = state_metadata();
        metadata.factory = None;
        metadata
            .methods
            .retain(|method| method.name != "unsubscribe");
        metadata.methods[0].return_type = "unit".to_string();

        let code = generate_swift_bridge_source(&metadata).unwrap();

        assert!(code.contains("public init()"));
        assert!(code.contains("let vm = CounterViewModel()"));
        assert!(code.contains("vm.subscribe(observer: observer)"));
        assert!(!code.contains("observerId"));
        assert!(!code.contains("deinit"));
    }

    #[test]
    fn list_bridge_handles_subscription_id_without_unsubscribe() {
        let mut metadata = list_metadata();
        metadata.methods[0].return_type = "i64".to_string();

        let code = generate_swift_bridge_source(&metadata).unwrap();

        assert!(code.contains("private var observerId: Int64?"));
        assert!(code.contains("self.observerId = vm.subscribe(observer: observer)"));
        assert!(!code.contains("deinit"));
    }

    #[test]
    fn reports_invalid_metadata_and_formats_errors() {
        let mut metadata = state_metadata();
        metadata.state_type = Some("WrongState".to_string());

        let err = generate_swift_bridge(&metadata).unwrap_err();
        assert_eq!(
            err.to_string(),
            "metadata method `get_state` is invalid: return type must match state_type"
        );

        let converted = CodegenError::from(MetadataValidationError::MissingField("observer"));
        assert_eq!(
            converted.to_string(),
            "metadata field `observer` is required"
        );
        assert_eq!(
            CodegenError::UnsupportedMode(VmMode::Event).to_string(),
            "unsupported bridge mode: Event"
        );
    }

    #[test]
    fn rejects_swift_specific_subscribe_and_unsubscribe_shape_mismatches() {
        let mut missing_observer_arg = list_metadata();
        missing_observer_arg.methods[0].args.clear();
        assert_eq!(
            generate_swift_bridge_source(&missing_observer_arg),
            Err(CodegenError::InvalidMetadata(
                MetadataValidationError::InvalidMethodShape {
                    method: "subscribe",
                    reason: "must accept exactly one observer argument"
                }
            ))
        );

        let mut wrong_unsubscribe_arg = state_metadata();
        let unsubscribe = wrong_unsubscribe_arg
            .methods
            .iter_mut()
            .find(|method| method.name == "unsubscribe")
            .unwrap();
        unsubscribe.args[0].name = "token".to_string();
        assert_eq!(
            generate_swift_bridge_source(&wrong_unsubscribe_arg),
            Err(CodegenError::InvalidMetadata(
                MetadataValidationError::InvalidMethodShape {
                    method: "unsubscribe",
                    reason: "must accept exactly one id argument"
                }
            ))
        );

        let mut wrong_subscribe_arg_type = state_metadata();
        wrong_subscribe_arg_type.methods[0].args[0].rust_type = "i32".to_string();
        assert_eq!(
            generate_swift_bridge_source(&wrong_subscribe_arg_type),
            Err(CodegenError::InvalidMetadata(
                MetadataValidationError::InvalidMethodShape {
                    method: "subscribe",
                    reason: "observer argument type must match observer"
                }
            ))
        );

        let mut wrong_unsubscribe_arg_type = state_metadata();
        let unsubscribe = wrong_unsubscribe_arg_type
            .methods
            .iter_mut()
            .find(|method| method.name == "unsubscribe")
            .unwrap();
        unsubscribe.args[0].rust_type = "String".to_string();
        assert_eq!(
            generate_swift_bridge_source(&wrong_unsubscribe_arg_type),
            Err(CodegenError::InvalidMetadata(
                MetadataValidationError::InvalidMethodShape {
                    method: "unsubscribe",
                    reason: "id argument type must match subscribe return type"
                }
            ))
        );

        let mut wrong_unsubscribe_return = state_metadata();
        let unsubscribe = wrong_unsubscribe_return
            .methods
            .iter_mut()
            .find(|method| method.name == "unsubscribe")
            .unwrap();
        unsubscribe.return_type = "bool".to_string();
        assert_eq!(
            generate_swift_bridge_source(&wrong_unsubscribe_return),
            Err(CodegenError::InvalidMetadata(
                MetadataValidationError::InvalidMethodShape {
                    method: "unsubscribe",
                    reason: "must return unit"
                }
            ))
        );
    }

    #[test]
    fn formats_internal_initializer_signature() {
        let method = MethodMetadata {
            name: "new".to_string(),
            args: vec![ArgMetadata {
                name: "initial_count".to_string(),
                rust_type: "i32".to_string(),
            }],
            return_type: "Arc<Self>".to_string(),
        };

        assert_eq!(
            format_swift_signature(&method, false),
            "init(initialCount: Int32)"
        );
    }
}
