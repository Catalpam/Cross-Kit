use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use serde_json::json;
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
    let swift_bridge = args.values.get("swift_bridge").cloned().unwrap_or_default();
    let mode = args.values.get("mode").cloned().unwrap_or_default();
    let observer = args.values.get("observer").cloned().unwrap_or_default();
    let observer_method = args.values.get("observer_method").cloned().unwrap_or_default();
    let state_type = args.values.get("state_type").cloned().unwrap_or_default();
    let diff_type = args.values.get("diff_type").cloned().unwrap_or_default();
    let list_item_type = args.values.get("list_item_type").cloned().unwrap_or_default();
    let factory_type = args.values.get("factory_type").cloned().unwrap_or_default();
    let factory_method = args
        .values
        .get("factory_method")
        .cloned()
        .unwrap_or_default();
    let factory_bridge = args
        .values
        .get("factory_bridge")
        .cloned()
        .unwrap_or_default();

    let input = parse_macro_input!(input as ItemImpl);
    let self_ty = &input.self_ty;
    let vm_type = quote!(#self_ty).to_string().replace(' ', "");

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
                    let arg_type = pat.ty.to_token_stream().to_string().replace(' ', "");
                    args.push(ArgInfo {
                        name: arg_name,
                        ty: arg_type,
                    });
                }
            }
            let ret = match &func.sig.output {
                syn::ReturnType::Default => "unit".to_string(),
                syn::ReturnType::Type(_, ty) => ty.to_token_stream().to_string().replace(' ', ""),
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

    let methods_json = methods
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
        .collect::<Vec<_>>();

    let meta = json!({
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
        "methods": methods_json,
        "swift_code": swift_code,
    })
    .to_string();

    let meta_literal = syn::LitStr::new(&meta, proc_macro2::Span::call_site());

    let expanded = quote! {
        #input

        impl CkVmMetadata for #self_ty {
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
        observer_id_unsubscribe = indent_optional(&observer_id_unsubscribe.trim_end().to_string(), 4),
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
        observer_id_unsubscribe = indent_optional(&observer_id_unsubscribe.trim_end().to_string(), 4),
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
        .map(|arg| format!("{}: {}", to_swift_method_name(&arg.name), to_swift_method_name(&arg.name)))
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
