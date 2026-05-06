use serde_json::Value;
use std::process::Command;

fn run_metadata_binary() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ck_shopping_cart_metadata"))
        .output()
        .expect("metadata binary should run");
    assert!(
        output.status.success(),
        "metadata binary should exit successfully: {:?}",
        output
    );
    assert!(
        output.stderr.is_empty(),
        "metadata binary should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("metadata stdout should be utf-8")
}

#[test]
fn metadata_json_is_valid_stdout_only_array() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn metadata_contains_shopping_cart_contract() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let items = parsed.as_array().unwrap();
    assert_eq!(
        items[0]["schema_version"],
        cross_kit::VM_METADATA_SCHEMA_VERSION
    );
    assert_eq!(
        items[1]["schema_version"],
        cross_kit::VM_METADATA_SCHEMA_VERSION
    );

    let ir: cross_kit::VmMetadata =
        serde_json::from_value(items[0]["ir"].clone()).expect("cart metadata ir should parse");
    ir.validate().expect("metadata ir should be valid");
    assert_eq!(ir.rust_type, "ShoppingCartViewModel");
    assert_eq!(ir.bridge_name, "ShoppingCartViewModelBridge");
    assert_eq!(ir.state_type.as_deref(), Some("ShoppingCartState"));
    assert!(ir.factory.is_none());

    let methods = ir
        .methods
        .iter()
        .map(|method| method.name.as_str())
        .collect::<Vec<_>>();
    assert!(methods.contains(&"new"));
    assert!(methods.contains(&"make_cart_vm"));
    assert!(methods.contains(&"apply_coupon"));
    assert!(methods.contains(&"clear_coupon"));
    assert!(methods.contains(&"checkout"));
    assert!(methods.contains(&"get_state"));
    assert!(methods.contains(&"subscribe"));
    assert!(methods.contains(&"unsubscribe"));

    let list_ir: cross_kit::VmMetadata =
        serde_json::from_value(items[1]["ir"].clone()).expect("list metadata ir should parse");
    list_ir
        .validate()
        .expect("list metadata ir should be valid");
    assert_eq!(list_ir.rust_type, "CartViewModel");
    assert_eq!(list_ir.mode, cross_kit::VmMode::DiffList);
    assert_eq!(list_ir.diff_type.as_deref(), Some("CartDiff"));
    assert_eq!(list_ir.list_item_type.as_deref(), Some("CartItem"));
    assert_eq!(
        list_ir
            .factory
            .as_ref()
            .map(|factory| factory.method.as_str()),
        Some("make_cart_vm")
    );
}

#[test]
fn metadata_ir_matches_shopping_cart_snapshot() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let generated_ir = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["ir"].clone())
        .collect::<Vec<_>>();
    let expected_ir: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/metadata/shopping-cart.json"
    ))
    .unwrap();

    assert_eq!(generated_ir, expected_ir);
}

#[test]
fn metadata_contains_swift_bridge_code() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let swift_code = parsed.as_array().unwrap()[0]
        .get("swift_code")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    assert!(swift_code.contains("public final class ShoppingCartViewModelBridge"));
    assert!(swift_code.contains("public func applyCoupon(code: String)"));
    assert!(swift_code.contains("public func checkout()"));
    let list_swift_code = parsed.as_array().unwrap()[1]
        .get("swift_code")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    assert!(list_swift_code.contains("public final class CartViewModelBridge"));
    assert!(list_swift_code.contains("public func addProduct(productId: Int64, quantity: Int64)"));
    assert!(list_swift_code.contains("public func setQuantity(productId: Int64, quantity: Int64)"));
    assert!(list_swift_code.contains("public func removeProduct(productId: Int64)"));
}
