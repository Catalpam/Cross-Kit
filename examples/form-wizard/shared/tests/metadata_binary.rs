use serde_json::Value;
use std::process::Command;

fn run_metadata_binary() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ck_form_wizard_metadata"))
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
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[test]
fn metadata_contains_form_wizard_contract() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let item = &parsed.as_array().unwrap()[0];
    assert_eq!(
        item["schema_version"],
        cross_kit::VM_METADATA_SCHEMA_VERSION
    );

    let ir: cross_kit::VmMetadata =
        serde_json::from_value(item["ir"].clone()).expect("metadata ir should parse");
    ir.validate().expect("metadata ir should be valid");
    assert_eq!(ir.rust_type, "FormWizardViewModel");
    assert_eq!(ir.bridge_name, "FormWizardViewModelBridge");
    assert_eq!(ir.state_type.as_deref(), Some("FormWizardState"));
    assert!(ir.factory.is_none());

    let methods = ir
        .methods
        .iter()
        .map(|method| method.name.as_str())
        .collect::<Vec<_>>();
    assert!(methods.contains(&"new"));
    assert!(methods.contains(&"update_name"));
    assert!(methods.contains(&"update_email"));
    assert!(methods.contains(&"update_password"));
    assert!(methods.contains(&"update_confirm"));
    assert!(methods.contains(&"next"));
    assert!(methods.contains(&"back"));
    assert!(methods.contains(&"get_state"));
    assert!(methods.contains(&"subscribe"));
    assert!(methods.contains(&"unsubscribe"));
}

#[test]
fn metadata_ir_matches_form_wizard_snapshot() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let generated_ir = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["ir"].clone())
        .collect::<Vec<_>>();
    let expected_ir: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/metadata/form-wizard.json"
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

    assert!(swift_code.contains("public final class FormWizardViewModelBridge"));
    assert!(swift_code.contains("public func updateName(name: String)"));
    assert!(swift_code.contains("public func updateEmail(email: String)"));
    assert!(swift_code.contains("public func updatePassword(password: String)"));
    assert!(swift_code.contains("public func updateConfirm(confirm: String)"));
    assert!(swift_code.contains("public func next()"));
    assert!(swift_code.contains("public func back()"));
}
