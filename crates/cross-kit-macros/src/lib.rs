use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use serde_json::{json, Value};
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{ImplItem, ItemImpl, LitStr, Path, PathArguments, Result as SynResult, Token};

struct MacroArgs {
    values: HashMap<String, String>,
    paths: HashMap<String, PathArg>,
}

#[derive(Clone)]
struct PathArg {
    path: Path,
    span: proc_macro2::Span,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut values = HashMap::new();
        let mut paths = HashMap::new();
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let key_name = key.to_string();
            input.parse::<Token![=]>()?;
            if is_path_arg_key(&key_name) {
                let path: Path = input.parse()?;
                paths.insert(
                    key_name,
                    PathArg {
                        span: path.span(),
                        path,
                    },
                );
            } else {
                let value: LitStr = input.parse()?;
                values.insert(key_name, value.value());
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { values, paths })
    }
}

/// Emits Cross-Kit VM bridge metadata for a UniFFI-exported Rust VM impl.
///
/// This proc macro is re-exported for users as [`cross_kit::vm_bridge`]. It
/// supports string options such as `mode = "state"` for compatibility and Rust
/// path options such as `factory = AppViewModel::make_counter_vm`,
/// `diff = ListDiff`, and `item = ListItem` for type-checked configuration.
///
/// The generated metadata is consumed by Cross-Kit's iOS and Android packagers;
/// the annotated impl itself is preserved unchanged for UniFFI.
///
/// [`cross_kit::vm_bridge`]: https://docs.rs/cross-kit/latest/cross_kit/attr.vm_bridge.html
#[proc_macro_attribute]
pub fn ck_vm_bridge(args: TokenStream, input: TokenStream) -> TokenStream {
    expand_ck_vm_bridge(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_ck_vm_bridge(
    args: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> SynResult<proc_macro2::TokenStream> {
    let args = syn::parse2::<MacroArgs>(args)?;
    let input = syn::parse2::<ItemImpl>(input)?;
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
        swift_bridge: arg_value(&args, &["bridge", "bridge_name", "swift_bridge"]),
        mode: args.values.get("mode").cloned().unwrap_or_default(),
        vm_type: vm_type.clone(),
        observer: args.values.get("observer").cloned().unwrap_or_default(),
        observer_method: args
            .values
            .get("observer_method")
            .cloned()
            .unwrap_or_default(),
        state_type: args.values.get("state_type").cloned().unwrap_or_default(),
        diff_type: args.values.get("diff_type").cloned().unwrap_or_default(),
        list_item_type: args
            .values
            .get("list_item_type")
            .cloned()
            .unwrap_or_default(),
        factory_type: args.values.get("factory_type").cloned().unwrap_or_default(),
        factory_method: args
            .values
            .get("factory_method")
            .cloned()
            .unwrap_or_default(),
        factory_bridge: arg_value(&args, &["factory_bridge", "factory_bridge_name"]),
        methods: methods.clone(),
    };
    let vm_meta = apply_path_args(&args, vm_meta)?;

    let (vm_meta, ir, swift_code) = match finalize_metadata(vm_meta) {
        Ok(value) => value,
        Err(message) => {
            return Err(syn::Error::new(proc_macro2::Span::call_site(), message));
        }
    };

    let meta = json!({
        "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
        "swift_bridge": vm_meta.swift_bridge,
        "mode": vm_meta.mode,
        "vm_type": vm_meta.vm_type,
        "observer": vm_meta.observer,
        "observer_method": vm_meta.observer_method,
        "state_type": vm_meta.state_type,
        "diff_type": vm_meta.diff_type,
        "list_item_type": vm_meta.list_item_type,
        "factory_type": vm_meta.factory_type,
        "factory_method": vm_meta.factory_method,
        "factory_bridge": vm_meta.factory_bridge,
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

    Ok(expanded)
}

#[derive(Clone, Debug)]
struct ArgInfo {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
struct MethodInfo {
    name: String,
    args: Vec<ArgInfo>,
    ret: String,
}

#[derive(Debug)]
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

fn is_path_arg_key(key: &str) -> bool {
    matches!(key, "factory" | "diff" | "item")
}

fn arg_value(args: &MacroArgs, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| args.values.get(*key))
        .cloned()
        .unwrap_or_default()
}

fn apply_path_args(args: &MacroArgs, mut meta: VmMetaLocal) -> SynResult<VmMetaLocal> {
    if let Some(factory) = args.paths.get("factory") {
        let (factory_type, factory_method) = parse_factory_path(factory)?;
        apply_path_value(
            &mut meta.factory_type,
            &factory_type,
            "factory",
            "factory_type",
            factory.span,
        )?;
        apply_path_value(
            &mut meta.factory_method,
            &factory_method,
            "factory",
            "factory_method",
            factory.span,
        )?;
    }
    if let Some(diff) = args.paths.get("diff") {
        let diff_type = parse_type_path(diff, "diff")?;
        apply_path_value(
            &mut meta.diff_type,
            &diff_type,
            "diff",
            "diff_type",
            diff.span,
        )?;
    }
    if let Some(item) = args.paths.get("item") {
        let list_item_type = parse_type_path(item, "item")?;
        apply_path_value(
            &mut meta.list_item_type,
            &list_item_type,
            "item",
            "list_item_type",
            item.span,
        )?;
    }
    Ok(meta)
}

fn parse_factory_path(arg: &PathArg) -> SynResult<(String, String)> {
    let segments = arg.path.segments.iter().collect::<Vec<_>>();
    if segments.len() != 2
        || segments
            .iter()
            .any(|segment| !path_segment_is_plain(segment))
    {
        return Err(syn::Error::new(
            arg.span,
            "factory path must be exactly `FactoryType::method`",
        ));
    }
    Ok((segments[0].ident.to_string(), segments[1].ident.to_string()))
}

fn parse_type_path(arg: &PathArg, key: &str) -> SynResult<String> {
    if arg.path.segments.is_empty()
        || arg
            .path
            .segments
            .iter()
            .any(|segment| !path_segment_is_plain(segment))
    {
        return Err(syn::Error::new(
            arg.span,
            format!("{key} path must be a plain Rust type path"),
        ));
    }
    Ok(platform_export_type_name(&normalize_rust_type(
        &arg.path.to_token_stream().to_string(),
    )))
}

fn path_segment_is_plain(segment: &syn::PathSegment) -> bool {
    matches!(segment.arguments, PathArguments::None)
}

fn apply_path_value(
    existing: &mut String,
    value: &str,
    path_key: &str,
    legacy_key: &str,
    span: proc_macro2::Span,
) -> SynResult<()> {
    if existing.trim().is_empty() {
        *existing = value.to_string();
        return Ok(());
    }
    if existing == value {
        return Ok(());
    }
    Err(syn::Error::new(
        span,
        format!("{path_key} conflicts with `{legacy_key}`: `{value}` != `{existing}`"),
    ))
}

fn finalize_metadata(mut meta: VmMetaLocal) -> Result<(VmMetaLocal, Value, String), String> {
    apply_defaults(&mut meta)?;
    let ir = metadata_ir_json(&meta);
    let metadata = serde_json::from_value::<cross_kit_core::VmMetadata>(ir.clone())
        .map_err(|err| format!("invalid vm_bridge metadata: {err}"))?;
    metadata
        .validate()
        .map_err(|err| format!("invalid vm_bridge metadata: {err}"))?;
    let swift_code = cross_kit_codegen::generate_swift_bridge_source(&metadata)
        .map_err(|err| format!("invalid vm_bridge metadata: {err}"))?;
    Ok((meta, ir, swift_code))
}

fn apply_defaults(meta: &mut VmMetaLocal) -> Result<(), String> {
    if meta.swift_bridge.trim().is_empty() {
        meta.swift_bridge = format!("{}Bridge", meta.vm_type);
    }
    if meta.observer.trim().is_empty() {
        if let Some(observer) = infer_observer_type(&meta.methods)? {
            meta.observer = observer;
        }
    }
    if meta.observer_method.trim().is_empty() {
        meta.observer_method = match meta.mode.as_str() {
            "state" => "on_state".to_string(),
            "diff_list" => "on_diffs".to_string(),
            "event" => "on_event".to_string(),
            _ => String::new(),
        };
    }
    if meta.mode == "state" && meta.state_type.trim().is_empty() {
        meta.state_type = infer_state_type(&meta.methods).ok_or_else(|| {
            "state VM requires get_state() with no arguments returning a non-unit state type, or an explicit state_type override"
                .to_string()
        })?;
    }
    Ok(())
}

fn infer_state_type(methods: &[MethodInfo]) -> Option<String> {
    methods
        .iter()
        .find(|method| method.name == "get_state" && method.args.is_empty())
        .map(|method| platform_export_type_name(&method.ret))
        .filter(|ret| ret != "unit")
}

fn infer_observer_type(methods: &[MethodInfo]) -> Result<Option<String>, String> {
    let Some(method) = methods.iter().find(|method| method.name == "subscribe") else {
        return Ok(None);
    };
    if method.args.len() != 1 || method.args[0].name != "observer" {
        return Err(
            "cannot infer observer: subscribe must accept exactly one `observer` argument"
                .to_string(),
        );
    }
    extract_observer_type(&method.args[0].ty)
        .map(Some)
        .ok_or_else(|| {
            "cannot infer observer: subscribe observer argument must be `Arc<dyn Observer>` or `std::sync::Arc<dyn Observer>`"
                .to_string()
        })
}

fn extract_observer_type(ty: &str) -> Option<String> {
    ty.strip_prefix("Arc<dyn ")
        .or_else(|| ty.strip_prefix("std::sync::Arc<dyn "))
        .and_then(|inner| inner.strip_suffix('>'))
        .map(str::trim)
        .filter(|inner| !inner.is_empty())
        .map(platform_export_type_name)
}

fn platform_export_type_name(ty: &str) -> String {
    let mut out = String::new();
    let mut token = String::new();
    for ch in ty.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            token.push(ch);
        } else {
            push_platform_type_token(&mut out, &mut token);
            out.push(ch);
        }
    }
    push_platform_type_token(&mut out, &mut token);
    out.trim().to_string()
}

fn push_platform_type_token(out: &mut String, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if token.contains("::") {
        out.push_str(token.rsplit("::").next().unwrap_or(token.as_str()));
    } else {
        out.push_str(token);
    }
    token.clear();
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
                .map(|arg| {
                    json!({"name": arg.name, "rust_type": platform_export_type_name(&arg.ty)})
                })
                .collect::<Vec<_>>();
            json!({
                "name": method.name,
                "args": args,
                "return_type": platform_export_type_name(&method.ret),
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

#[cfg(test)]
fn swift_code_from_ir(ir: &Value) -> Result<String, String> {
    serde_json::from_value::<cross_kit_core::VmMetadata>(ir.clone())
        .map_err(|err| format!("invalid vm_bridge metadata: {err}"))
        .and_then(|metadata| {
            metadata
                .validate()
                .map_err(|err| format!("invalid vm_bridge metadata: {err}"))?;
            cross_kit_codegen::generate_swift_bridge_source(&metadata)
                .map_err(|err| format!("invalid vm_bridge metadata: {err}"))
        })
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

    fn minimal_state_meta() -> VmMetaLocal {
        VmMetaLocal {
            swift_bridge: String::new(),
            mode: "state".to_string(),
            vm_type: "CounterViewModel".to_string(),
            observer: String::new(),
            observer_method: String::new(),
            state_type: String::new(),
            diff_type: String::new(),
            list_item_type: String::new(),
            factory_type: String::new(),
            factory_method: String::new(),
            factory_bridge: String::new(),
            methods: vec![
                MethodInfo {
                    name: "subscribe".to_string(),
                    args: vec![ArgInfo {
                        name: "observer".to_string(),
                        ty: "std::sync::Arc<dyn CounterObserver>".to_string(),
                    }],
                    ret: "i64".to_string(),
                },
                MethodInfo {
                    name: "get_state".to_string(),
                    args: Vec::new(),
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
    fn parses_string_macro_args_with_trailing_comma_and_aliases() {
        let args: MacroArgs = syn::parse2(quote!(
            mode = "state",
            bridge_name = "CounterBridge",
            factory_bridge_name = "AppBridge",
        ))
        .unwrap();

        assert_eq!(args.values["mode"], "state");
        assert_eq!(
            arg_value(&args, &["bridge", "bridge_name"]),
            "CounterBridge"
        );
        assert_eq!(
            arg_value(&args, &["factory_bridge", "factory_bridge_name"]),
            "AppBridge"
        );
    }

    #[test]
    fn parses_mixed_string_and_path_macro_args() {
        let args: MacroArgs = syn::parse2(quote!(
            mode = "state",
            bridge = "CustomCounterBridge",
            factory = AppViewModel::make_counter_vm,
            diff = types::ListDiff,
            item = types::ListItem,
        ))
        .unwrap();

        assert_eq!(args.values["mode"], "state");
        assert_eq!(args.values["bridge"], "CustomCounterBridge");
        assert_eq!(
            normalize_rust_type(&args.paths["factory"].path.to_token_stream().to_string()),
            "AppViewModel::make_counter_vm"
        );
        assert_eq!(
            parse_type_path(&args.paths["diff"], "diff").unwrap(),
            "ListDiff"
        );
        assert_eq!(
            parse_type_path(&args.paths["item"], "item").unwrap(),
            "ListItem"
        );
    }

    #[test]
    fn path_args_merge_with_legacy_values_and_report_conflicts() {
        let args: MacroArgs = syn::parse2(quote!(
            factory = AppViewModel::make_counter_vm,
            factory_type = "AppViewModel",
            factory_method = "make_counter_vm",
            diff = ListDiff,
            diff_type = "ListDiff",
            item = ListItem,
            list_item_type = "ListItem",
        ))
        .unwrap();
        let mut legacy_meta = list_meta();
        legacy_meta.factory_type = "AppViewModel".to_string();
        legacy_meta.factory_method = "make_counter_vm".to_string();
        let meta = apply_path_args(&args, legacy_meta).unwrap();

        assert_eq!(meta.factory_type, "AppViewModel");
        assert_eq!(meta.factory_method, "make_counter_vm");
        assert_eq!(meta.diff_type, "ListDiff");
        assert_eq!(meta.list_item_type, "ListItem");

        let args: MacroArgs = syn::parse2(quote!(
            factory = AppViewModel::make_counter_vm,
            factory_type = "OtherViewModel",
        ))
        .unwrap();
        let mut legacy_meta = list_meta();
        legacy_meta.factory_type = "OtherViewModel".to_string();
        let error = apply_path_args(&args, legacy_meta).unwrap_err();
        assert!(error
            .to_string()
            .contains("factory conflicts with `factory_type`"));
    }

    #[test]
    fn factory_path_rejects_unsupported_shapes() {
        let one_segment: MacroArgs = syn::parse2(quote!(factory = AppViewModel)).unwrap();
        assert!(parse_factory_path(&one_segment.paths["factory"])
            .unwrap_err()
            .to_string()
            .contains("FactoryType::method"));

        let nested: MacroArgs =
            syn::parse2(quote!(factory = AppViewModel::nested::make_counter_vm)).unwrap();
        assert!(parse_factory_path(&nested.paths["factory"])
            .unwrap_err()
            .to_string()
            .contains("FactoryType::method"));
    }

    #[test]
    fn expand_vm_bridge_builds_metadata_impl_and_ignores_private_methods() {
        let expanded = expand_ck_vm_bridge(
            quote!(mode = "state"),
            quote! {
                impl CounterViewModel {
                    fn private_helper(&self) -> bool {
                        true
                    }

                    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64 {
                        drop(observer);
                        1
                    }

                    pub fn get_state(&self) -> CounterState {
                        CounterState { value: 0 }
                    }

                    pub fn reset(&self) {}
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(expanded.contains("impl cross_kit :: CkVmMetadata for CounterViewModel"));
        assert!(expanded.contains("CounterViewModelBridge"));
        assert!(expanded.contains("CounterObserver"));
    }

    #[test]
    fn expand_vm_bridge_falls_back_to_arg_for_non_ident_patterns() {
        let expanded = expand_ck_vm_bridge(
            quote!(mode = "state", state_type = "CounterState"),
            quote! {
                impl CounterViewModel {
                    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64 {
                        drop(observer);
                        1
                    }

                    pub fn get_state(&self) -> CounterState {
                        CounterState { value: 0 }
                    }

                    pub fn replace_state(&self, (next, _): (CounterState, bool)) -> CounterState {
                        next
                    }
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(expanded.contains("\\\"name\\\":\\\"arg\\\""));
    }

    #[test]
    fn expand_vm_bridge_returns_compile_error_for_invalid_metadata() {
        let error = expand_ck_vm_bridge(
            quote!(mode = "state"),
            quote! {
                impl BrokenViewModel {
                    pub fn subscribe(&self, callback: Arc<dyn CounterObserver>) -> i64 {
                        drop(callback);
                        1
                    }
                }
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("subscribe must accept exactly one `observer` argument"));
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
    fn platform_export_type_name_strips_qualified_segments_inside_generics() {
        assert_eq!(
            platform_export_type_name("crate::models::CounterState"),
            "CounterState"
        );
        assert_eq!(
            platform_export_type_name("Vec<crate::models::ListDiff>"),
            "Vec<ListDiff>"
        );
        assert_eq!(
            platform_export_type_name("std::sync::Arc<dyn crate::observers::CounterObserver>"),
            "Arc<dyn CounterObserver>"
        );
    }

    #[test]
    fn compatibility_swift_code_is_delegated_from_ir() {
        let ir = metadata_ir_json(&state_meta());
        let code = swift_code_from_ir(&ir).unwrap();

        assert!(code.contains("// Generated by cross-kit-codegen."));
        assert!(code.contains("public final class CounterViewModelBridge"));
    }

    #[test]
    fn infers_state_vm_defaults_from_impl_methods() {
        let (meta, ir, swift_code) = finalize_metadata(minimal_state_meta()).unwrap();
        let metadata: cross_kit_core::VmMetadata = serde_json::from_value(ir).unwrap();

        assert_eq!(meta.swift_bridge, "CounterViewModelBridge");
        assert_eq!(meta.observer, "CounterObserver");
        assert_eq!(meta.observer_method, "on_state");
        assert_eq!(meta.state_type, "CounterState");
        assert_eq!(metadata.bridge_name, "CounterViewModelBridge");
        assert_eq!(metadata.state_type.as_deref(), Some("CounterState"));
        assert!(swift_code.contains("public final class CounterViewModelBridge"));
    }

    #[test]
    fn inferred_platform_types_strip_rust_module_qualifiers() {
        let mut meta = minimal_state_meta();
        meta.methods[0].args[0].ty =
            "std::sync::Arc<dyn crate::observers::CounterObserver>".to_string();
        meta.methods[1].ret = "crate::models::CounterState".to_string();

        let (meta, ir, _) = finalize_metadata(meta).unwrap();
        let metadata: cross_kit_core::VmMetadata = serde_json::from_value(ir).unwrap();

        assert_eq!(meta.observer, "CounterObserver");
        assert_eq!(meta.state_type, "CounterState");
        assert_eq!(
            metadata.observer.as_ref().unwrap().rust_type,
            "CounterObserver"
        );
        assert_eq!(metadata.state_type.as_deref(), Some("CounterState"));
    }

    #[test]
    fn infers_diff_list_observer_defaults_but_requires_diff_types() {
        let mut meta = list_meta();
        meta.observer.clear();
        meta.observer_method.clear();
        meta.methods[0].args = vec![ArgInfo {
            name: "observer".to_string(),
            ty: "Arc<dyn ListObserver>".to_string(),
        }];
        meta.diff_type.clear();

        let error = finalize_metadata(meta).unwrap_err();
        assert!(error.contains("metadata field `diff_type` is required"));
    }

    #[test]
    fn reports_invalid_subscribe_shape_during_macro_metadata_finalization() {
        let mut meta = minimal_state_meta();
        meta.methods[0].args[0].name = "callback".to_string();

        let error = finalize_metadata(meta).unwrap_err();
        assert!(error.contains("subscribe must accept exactly one `observer` argument"));
    }

    #[test]
    fn defaults_event_observer_method_and_keeps_observer_null_when_absent() {
        let mut meta = minimal_state_meta();
        meta.mode = "event".to_string();
        meta.observer.clear();
        meta.observer_method.clear();
        meta.state_type.clear();
        meta.methods.clear();

        apply_defaults(&mut meta).unwrap();
        assert_eq!(meta.observer_method, "on_event");

        meta.observer_method.clear();
        assert!(optional_observer_json(&meta).is_null());
    }

    #[test]
    fn unknown_mode_does_not_default_observer_method_before_validation() {
        let mut meta = minimal_state_meta();
        meta.mode = "custom".to_string();
        meta.observer.clear();
        meta.observer_method.clear();
        meta.state_type.clear();
        meta.methods.clear();

        apply_defaults(&mut meta).unwrap();
        assert!(meta.observer_method.is_empty());
    }

    #[test]
    fn state_defaults_require_inferable_getter() {
        let mut meta = minimal_state_meta();
        meta.state_type.clear();
        meta.methods.retain(|method| method.name != "get_state");

        let error = apply_defaults(&mut meta).unwrap_err();
        assert!(error.contains("state VM requires get_state()"));
    }

    #[test]
    fn observer_inference_returns_none_when_subscribe_is_absent() {
        let mut meta = minimal_state_meta();
        meta.methods.retain(|method| method.name != "subscribe");

        assert!(infer_observer_type(&meta.methods).unwrap().is_none());
    }

    #[test]
    fn observer_inference_rejects_empty_dyn_target() {
        assert!(extract_observer_type("Arc<dyn >").is_none());
    }

    #[test]
    fn observer_inference_rejects_unsupported_pointer_shape() {
        let methods = vec![MethodInfo {
            name: "subscribe".to_string(),
            args: vec![ArgInfo {
                name: "observer".to_string(),
                ty: "Box<dyn CounterObserver>".to_string(),
            }],
            ret: "i64".to_string(),
        }];

        let error = infer_observer_type(&methods).unwrap_err();
        assert!(error.contains("Arc<dyn Observer>"));
    }

    #[test]
    fn finalize_metadata_reports_codegen_validation_errors_without_empty_swift_code() {
        let mut meta = minimal_state_meta();
        meta.state_type = "WrongState".to_string();

        let error = finalize_metadata(meta).unwrap_err();
        assert!(error.contains("get_state"));
        assert!(error.contains("return type must match state_type"));
    }
}
