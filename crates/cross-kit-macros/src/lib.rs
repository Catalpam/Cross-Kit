use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use serde_json::{json, Value};
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, ImplItem, ItemImpl, LitStr, Result as SynResult, Token};

struct MacroArgs {
    values: HashMap<String, String>,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut values = HashMap::new();
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            values.insert(key.to_string(), value.value());
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { values })
    }
}

#[proc_macro_attribute]
pub fn ck_vm_bridge(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as MacroArgs);
    let swift_bridge = arg_value(&args, &["bridge", "bridge_name", "swift_bridge"]);
    let mode = args.values.get("mode").cloned().unwrap_or_default();
    let observer = args.values.get("observer").cloned().unwrap_or_default();
    let observer_method = args
        .values
        .get("observer_method")
        .cloned()
        .unwrap_or_default();
    let state_type = args.values.get("state_type").cloned().unwrap_or_default();
    let diff_type = args.values.get("diff_type").cloned().unwrap_or_default();
    let list_item_type = args
        .values
        .get("list_item_type")
        .cloned()
        .unwrap_or_default();
    let factory_type = args.values.get("factory_type").cloned().unwrap_or_default();
    let factory_method = args
        .values
        .get("factory_method")
        .cloned()
        .unwrap_or_default();
    let factory_bridge = arg_value(&args, &["factory_bridge", "factory_bridge_name"]);

    let input = parse_macro_input!(input as ItemImpl);
    let self_ty = &input.self_ty;
    let vm_type = normalize_rust_type(&quote!(#self_ty).to_string());

    let mut methods = Vec::new();
    for item in &input.items {
        if let ImplItem::Fn(func) = item {
            if !matches!(func.vis, syn::Visibility::Public(_)) {
                continue;
            }
            let name = func.sig.ident.to_string();
            let mut args = Vec::new();
            for arg in &func.sig.inputs {
                if let syn::FnArg::Typed(pat) = arg {
                    let arg_name = match &*pat.pat {
                        syn::Pat::Ident(ident) => ident.ident.to_string(),
                        _ => "arg".to_string(),
                    };
                    let arg_type = normalize_rust_type(&pat.ty.to_token_stream().to_string());
                    args.push(ArgInfo {
                        name: arg_name,
                        ty: arg_type,
                    });
                }
            }
            let ret = match &func.sig.output {
                syn::ReturnType::Default => "unit".to_string(),
                syn::ReturnType::Type(_, ty) => {
                    normalize_rust_type(&ty.to_token_stream().to_string())
                }
            };
            methods.push(MethodInfo { name, args, ret });
        }
    }

    let swift_code = generate_swift_bridge(&VmMetaLocal {
        swift_bridge: swift_bridge.clone(),
        mode: mode.clone(),
        vm_type: vm_type.clone(),
        observer: observer.clone(),
        observer_method: observer_method.clone(),
        state_type: state_type.clone(),
        diff_type: diff_type.clone(),
        list_item_type: list_item_type.clone(),
        factory_type: factory_type.clone(),
        factory_method: factory_method.clone(),
        factory_bridge: factory_bridge.clone(),
        methods: methods.clone(),
    });

    let vm_meta = VmMetaLocal {
        swift_bridge: swift_bridge.clone(),
        mode: mode.clone(),
        vm_type: vm_type.clone(),
        observer: observer.clone(),
        observer_method: observer_method.clone(),
        state_type: state_type.clone(),
        diff_type: diff_type.clone(),
        list_item_type: list_item_type.clone(),
        factory_type: factory_type.clone(),
        factory_method: factory_method.clone(),
        factory_bridge: factory_bridge.clone(),
        methods: methods.clone(),
    };

    let meta = json!({
        "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
        "swift_bridge": swift_bridge,
        "mode": mode,
        "vm_type": vm_type,
        "observer": observer,
        "observer_method": observer_method,
        "state_type": state_type,
        "diff_type": diff_type,
        "list_item_type": list_item_type,
        "factory_type": factory_type,
        "factory_method": factory_method,
        "factory_bridge": factory_bridge,
        "methods": legacy_methods_json(&methods),
        "ir": metadata_ir_json(&vm_meta),
        "swift_code": swift_code,
    })
    .to_string();

    let meta_literal = syn::LitStr::new(&meta, proc_macro2::Span::call_site());

    let expanded = quote! {
        #input

        impl cross_kit::CkVmMetadata for #self_ty {
            fn ck_vm_metadata() -> &'static str {
                #meta_literal
            }
        }
    };

    expanded.into()
}

#[derive(Clone)]
struct ArgInfo {
    name: String,
    ty: String,
}

#[derive(Clone)]
struct MethodInfo {
    name: String,
    args: Vec<ArgInfo>,
    ret: String,
}

struct VmMetaLocal {
    swift_bridge: String,
    mode: String,
    vm_type: String,
    observer: String,
    observer_method: String,
    state_type: String,
    diff_type: String,
    list_item_type: String,
    factory_type: String,
    factory_method: String,
    factory_bridge: String,
    methods: Vec<MethodInfo>,
}

fn arg_value(args: &MacroArgs, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| args.values.get(*key))
        .cloned()
        .unwrap_or_default()
}

fn metadata_ir_json(meta: &VmMetaLocal) -> Value {
    json!({
        "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
        "rust_type": meta.vm_type,
        "bridge_name": meta.swift_bridge,
        "mode": meta.mode,
        "observer": optional_observer_json(meta),
        "state_type": optional_string_json(&meta.state_type),
        "diff_type": optional_string_json(&meta.diff_type),
        "list_item_type": optional_string_json(&meta.list_item_type),
        "factory": optional_factory_json(meta),
        "methods": ir_methods_json(&meta.methods),
    })
}

fn legacy_methods_json(methods: &[MethodInfo]) -> Vec<Value> {
    methods
        .iter()
        .map(|method| {
            let args = method
                .args
                .iter()
                .map(|arg| json!({"name": arg.name, "ty": arg.ty}))
                .collect::<Vec<_>>();
            json!({
                "name": method.name,
                "args": args,
                "ret": method.ret,
            })
        })
        .collect()
}

fn ir_methods_json(methods: &[MethodInfo]) -> Vec<Value> {
    methods
        .iter()
        .map(|method| {
            let args = method
                .args
                .iter()
                .map(|arg| json!({"name": arg.name, "rust_type": arg.ty}))
                .collect::<Vec<_>>();
            json!({
                "name": method.name,
                "args": args,
                "return_type": method.ret,
            })
        })
        .collect()
}

fn optional_observer_json(meta: &VmMetaLocal) -> Value {
    if meta.observer.trim().is_empty() && meta.observer_method.trim().is_empty() {
        Value::Null
    } else {
        json!({
            "rust_type": meta.observer,
            "method": meta.observer_method,
        })
    }
}

fn optional_factory_json(meta: &VmMetaLocal) -> Value {
    if meta.factory_type.trim().is_empty() && meta.factory_method.trim().is_empty() {
        Value::Null
    } else {
        json!({
            "rust_type": meta.factory_type,
            "method": meta.factory_method,
            "bridge_name": resolved_factory_bridge_name(meta),
        })
    }
}

fn optional_string_json(value: &str) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        json!(value)
    }
}

fn resolved_factory_bridge_name(meta: &VmMetaLocal) -> String {
    if meta.factory_bridge.trim().is_empty() && !meta.factory_type.trim().is_empty() {
        format!("{}Bridge", meta.factory_type)
    } else {
        meta.factory_bridge.clone()
    }
}

fn normalize_rust_type(input: &str) -> String {
    let mut out = input.trim().to_string();
    for (from, to) in [
        (" :: ", "::"),
        (" < ", "<"),
        (" >", ">"),
        (" , ", ", "),
        (" ( ", "("),
        (" ) ", ")"),
        (" [ ", "["),
        (" ] ", "]"),
        (" & ", "&"),
    ] {
        out = out.replace(from, to);
    }
    out
}

fn generate_swift_bridge(meta: &VmMetaLocal) -> String {
    match meta.mode.as_str() {
        "state" => generate_state_bridge(meta),
        "diff_list" => generate_list_bridge(meta),
        _ => String::new(),
    }
}

struct SubscribeInfo {
    ret_type: String,
}

fn generate_state_bridge(meta: &VmMetaLocal) -> String {
    let (ctor_sig, ctor_call) = constructor_or_factory(meta);
    let subscribe_info = subscribe_info(meta);
    let unsubscribe_name = unsubscribe_method_name(meta);
    let observer_method = to_swift_method_name(&meta.observer_method);

    let mut methods = String::new();
    for method in filtered_methods(meta) {
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
        observer = meta.observer,
        bridge = meta.swift_bridge,
        observer_method = observer_method,
        state_type = meta.state_type
    );

    let (observer_id_decl, observer_id_assign, observer_id_unsubscribe) = subscribe_info
        .as_ref()
        .map(|info| {
            let decl = format!("private var observerId: {}?", info.ret_type);
            let assign = format!("self.observerId = vm.subscribe(observer: observer)");
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
        r#"// Generated by ck-vm-macros.
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
        bridge = meta.swift_bridge,
        observer = meta.observer,
        state_type = meta.state_type,
        vm_type = meta.vm_type,
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

fn generate_list_bridge(meta: &VmMetaLocal) -> String {
    let (ctor_sig, ctor_call) = constructor_or_factory(meta);
    let subscribe_info = subscribe_info(meta);
    let unsubscribe_name = unsubscribe_method_name(meta);
    let observer_method = to_swift_method_name(&meta.observer_method);

    let mut methods = String::new();
    for method in filtered_methods(meta) {
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
        observer = meta.observer,
        bridge = meta.swift_bridge,
        observer_method = observer_method,
        diff_type = meta.diff_type
    );

    let (observer_id_decl, observer_id_assign, observer_id_unsubscribe) = subscribe_info
        .as_ref()
        .map(|info| {
            let decl = format!("private var observerId: {}?", info.ret_type);
            let assign = format!("self.observerId = vm.subscribe(observer: observer)");
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
        r#"// Generated by ck-vm-macros.
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
                let adjusted = fromIdx < toIdx ? toIdx - 1 : toIdx
                items.insert(item, at: adjusted)
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
        bridge = meta.swift_bridge,
        observer = meta.observer,
        list_item = meta.list_item_type,
        vm_type = meta.vm_type,
        observer_method = observer_method,
        diff_type = meta.diff_type,
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

fn filtered_methods(meta: &VmMetaLocal) -> Vec<&MethodInfo> {
    meta.methods
        .iter()
        .filter(|method| method.name != "subscribe" && method.name != "new")
        .filter(|method| method.name != "unsubscribe")
        .collect()
}

fn constructor_method(meta: &VmMetaLocal) -> Option<&MethodInfo> {
    meta.methods.iter().find(|method| method.name == "new")
}

fn constructor_or_factory(meta: &VmMetaLocal) -> (String, String) {
    if !meta.factory_method.is_empty() {
        let bridge = if meta.factory_bridge.is_empty() {
            format!("{}Bridge", meta.factory_type)
        } else {
            meta.factory_bridge.clone()
        };
        let method = to_swift_method_name(&meta.factory_method);
        return (
            format!("public init(app: {bridge})"),
            format!("app.{method}()"),
        );
    }
    if let Some(ctor) = constructor_method(meta) {
        return (
            format_swift_signature(ctor, true),
            format_swift_ctor_call(ctor, &meta.vm_type),
        );
    }
    ("public init()".to_string(), format!("{}()", meta.vm_type))
}

fn subscribe_info(meta: &VmMetaLocal) -> Option<SubscribeInfo> {
    let method = meta.methods.iter().find(|m| m.name == "subscribe")?;
    let ret_type = map_type_to_swift(&method.ret);
    if ret_type == "Void" {
        None
    } else {
        Some(SubscribeInfo { ret_type })
    }
}

fn unsubscribe_method_name(meta: &VmMetaLocal) -> Option<String> {
    meta.methods
        .iter()
        .find(|m| m.name == "unsubscribe")
        .map(|m| to_swift_method_name(&m.name))
}

fn format_swift_signature(method: &MethodInfo, with_public: bool) -> String {
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_swift_method_name(&arg.name),
                map_type_to_swift(&arg.ty)
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

fn format_swift_ctor_call(method: &MethodInfo, vm_type: &str) -> String {
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

fn format_swift_method(method: &MethodInfo) -> String {
    let swift_name = to_swift_method_name(&method.name);
    let args = method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}",
                to_swift_method_name(&arg.name),
                map_type_to_swift(&arg.ty)
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
    let ret_type = map_type_to_swift(&method.ret);
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

fn map_type_to_swift(ty: &str) -> String {
    let ty = ty.trim();
    if ty == "unit" {
        return "Void".to_string();
    }
    if let Some(inner) = ty.strip_prefix("Arc<").and_then(|v| v.strip_suffix('>')) {
        return format!("{}Protocol", map_type_to_swift(inner));
    }
    if let Some(inner) = ty
        .strip_prefix("std::sync::Arc<")
        .and_then(|v| v.strip_suffix('>'))
    {
        return format!("{}Protocol", map_type_to_swift(inner));
    }
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|v| v.strip_suffix('>')) {
        return format!("{}?", map_type_to_swift(inner));
    }
    if let Some(inner) = ty.strip_prefix("Vec<").and_then(|v| v.strip_suffix('>')) {
        return format!("[{}]", map_type_to_swift(inner));
    }
    match ty {
        "i64" => "Int64".to_string(),
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

    fn state_meta() -> VmMetaLocal {
        VmMetaLocal {
            swift_bridge: "CounterViewModelBridge".to_string(),
            mode: "state".to_string(),
            vm_type: "CounterViewModel".to_string(),
            observer: "CounterObserver".to_string(),
            observer_method: "on_state".to_string(),
            state_type: "CounterState".to_string(),
            diff_type: String::new(),
            list_item_type: String::new(),
            factory_type: "AppViewModel".to_string(),
            factory_method: "make_counter_vm".to_string(),
            factory_bridge: "AppViewModelBridge".to_string(),
            methods: vec![
                MethodInfo {
                    name: "subscribe".to_string(),
                    args: vec![ArgInfo {
                        name: "observer".to_string(),
                        ty: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    ret: "i64".to_string(),
                },
                MethodInfo {
                    name: "unsubscribe".to_string(),
                    args: vec![ArgInfo {
                        name: "id".to_string(),
                        ty: "i64".to_string(),
                    }],
                    ret: "unit".to_string(),
                },
                MethodInfo {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    ret: "CounterState".to_string(),
                },
                MethodInfo {
                    name: "increment_by".to_string(),
                    args: vec![ArgInfo {
                        name: "delta_value".to_string(),
                        ty: "i32".to_string(),
                    }],
                    ret: "CounterState".to_string(),
                },
            ],
        }
    }

    fn list_meta() -> VmMetaLocal {
        VmMetaLocal {
            swift_bridge: "ListViewModelBridge".to_string(),
            mode: "diff_list".to_string(),
            vm_type: "ListViewModel".to_string(),
            observer: "ListObserver".to_string(),
            observer_method: "on_diffs".to_string(),
            state_type: String::new(),
            diff_type: "ListDiff".to_string(),
            list_item_type: "ListItem".to_string(),
            factory_type: String::new(),
            factory_method: String::new(),
            factory_bridge: String::new(),
            methods: vec![
                MethodInfo {
                    name: "subscribe".to_string(),
                    args: Vec::new(),
                    ret: "unit".to_string(),
                },
                MethodInfo {
                    name: "append_now".to_string(),
                    args: Vec::new(),
                    ret: "ListItem".to_string(),
                },
                MethodInfo {
                    name: "apply_diffs".to_string(),
                    args: vec![ArgInfo {
                        name: "diffs".to_string(),
                        ty: "Vec<ListDiff>".to_string(),
                    }],
                    ret: "bool".to_string(),
                },
            ],
        }
    }

    #[test]
    fn maps_rust_types_to_swift_types() {
        assert_eq!(map_type_to_swift("unit"), "Void");
        assert_eq!(map_type_to_swift("i64"), "Int64");
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
            map_type_to_swift("std::sync::Arc<ListViewModel>"),
            "ListViewModelProtocol"
        );
        assert_eq!(map_type_to_swift("CustomRecord"), "CustomRecord");
    }

    #[test]
    fn converts_snake_case_to_swift_method_names() {
        assert_eq!(to_swift_method_name("get_state"), "getState");
        assert_eq!(to_swift_method_name("append_now"), "appendNow");
        assert_eq!(to_swift_method_name("already"), "already");
        assert_eq!(to_swift_method_name(""), "");
    }

    #[test]
    fn generates_state_bridge_with_factory_and_unsubscribe() {
        let code = generate_swift_bridge(&state_meta());

        assert!(code.contains("public final class CounterViewModelBridge"));
        assert!(code.contains("public init(app: AppViewModelBridge)"));
        assert!(code.contains("let vm = app.makeCounterVm()"));
        assert!(code.contains("private var observerId: Int64?"));
        assert!(code.contains("vm.unsubscribe(id: id)"));
        assert!(code.contains("public func incrementBy(deltaValue: Int32) -> CounterState"));
        assert!(!code.contains("public func subscribe"));
    }

    #[test]
    fn emits_versioned_target_independent_ir_for_state_vm() {
        let ir = metadata_ir_json(&state_meta());
        let metadata: cross_kit_core::VmMetadata = serde_json::from_value(ir).unwrap();

        assert_eq!(
            metadata.schema_version,
            cross_kit_core::VM_METADATA_SCHEMA_VERSION
        );
        assert_eq!(metadata.rust_type, "CounterViewModel");
        assert_eq!(metadata.bridge_name, "CounterViewModelBridge");
        assert_eq!(metadata.mode, cross_kit_core::VmMode::State);
        assert_eq!(
            metadata.factory.as_ref().unwrap().bridge_name,
            "AppViewModelBridge"
        );
        assert_eq!(
            metadata.methods[3].args[0].rust_type, "i32",
            "IR args use rust_type instead of the legacy ty field"
        );
        assert!(
            serde_json::to_value(&metadata)
                .unwrap()
                .get("swift_code")
                .is_none(),
            "target-independent IR must not contain generated platform source"
        );
        metadata.validate().unwrap();
    }

    #[test]
    fn generates_diff_list_bridge_with_diff_application() {
        let code = generate_swift_bridge(&list_meta());

        assert!(code.contains("@Published public private(set) var items: [ListItem] = []"));
        assert!(code.contains("public func appendNow() -> ListItem"));
        assert!(code.contains("public func applyDiffs(diffs: [ListDiff]) -> Bool"));
        assert!(code.contains("case let .move(from, to):"));
        assert!(code.contains("private func clampIndex"));
        assert!(code.contains("vm.subscribe(observer: observer)"));
    }

    #[test]
    fn emits_versioned_ir_for_diff_list_vm_without_factory() {
        let ir = metadata_ir_json(&list_meta());
        let metadata: cross_kit_core::VmMetadata = serde_json::from_value(ir).unwrap();

        assert_eq!(metadata.mode, cross_kit_core::VmMode::DiffList);
        assert_eq!(metadata.diff_type.as_deref(), Some("ListDiff"));
        assert_eq!(metadata.list_item_type.as_deref(), Some("ListItem"));
        assert!(metadata.factory.is_none());
        assert_eq!(metadata.methods[2].return_type, "bool");
        metadata.validate().unwrap();
    }

    #[test]
    fn keeps_legacy_method_shape_for_existing_packager() {
        let methods = legacy_methods_json(&state_meta().methods);

        assert_eq!(methods[0]["args"][0]["ty"], "Arc<dyn CounterObserver>");
        assert_eq!(methods[0]["ret"], "i64");
        assert!(methods[0]["args"][0].get("rust_type").is_none());
    }

    #[test]
    fn resolves_factory_bridge_name_for_ir_when_attribute_is_omitted() {
        let mut meta = state_meta();
        meta.factory_bridge.clear();

        let ir = metadata_ir_json(&meta);
        assert_eq!(ir["factory"]["bridge_name"], "AppViewModelBridge");
    }

    #[test]
    fn unknown_bridge_mode_generates_empty_code() {
        let mut meta = state_meta();
        meta.mode = "unknown".to_string();
        assert!(generate_swift_bridge(&meta).is_empty());
    }

    #[test]
    fn formats_method_signatures_and_indentation() {
        let method = MethodInfo {
            name: "clear_route".to_string(),
            args: Vec::new(),
            ret: "unit".to_string(),
        };
        assert_eq!(
            format_swift_method(&method),
            "public func clearRoute() {\n        vm.clearRoute()\n    }\n\n"
        );
        assert_eq!(indent("a\n\nb", 2), "  a\n\n  b");
        assert_eq!(indent_optional("  ", 4), "");
        assert_eq!(indent_optional("x", 4), "    x");
    }

    #[test]
    fn normalizes_rust_type_tokens_without_dropping_dyn_space() {
        assert_eq!(
            normalize_rust_type("Arc < dyn CounterObserver >"),
            "Arc<dyn CounterObserver>"
        );
        assert_eq!(
            normalize_rust_type("std :: sync :: Arc < ListViewModel >"),
            "std::sync::Arc<ListViewModel>"
        );
        assert_eq!(
            normalize_rust_type("Option < Vec < ListDiff > >"),
            "Option<Vec<ListDiff>>"
        );
    }
}
