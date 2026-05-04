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
    let ir = metadata_ir_json(&vm_meta);
    let swift_code = swift_code_from_ir(&ir);

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
        "ir": ir,
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

fn swift_code_from_ir(ir: &Value) -> String {
    serde_json::from_value::<cross_kit_core::VmMetadata>(ir.clone())
        .ok()
        .and_then(|metadata| cross_kit_codegen::generate_swift_bridge_source(&metadata).ok())
        .unwrap_or_default()
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

    #[test]
    fn compatibility_swift_code_is_delegated_from_ir() {
        let ir = metadata_ir_json(&state_meta());
        let code = swift_code_from_ir(&ir);

        assert!(code.contains("// Generated by cross-kit-codegen."));
        assert!(code.contains("public final class CounterViewModelBridge"));
    }
}
