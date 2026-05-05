use serde_json::Value;
use std::process::Command;

fn run_metadata_binary() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ck_vm_metadata"))
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
}

#[test]
fn metadata_contains_swift_code() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let items = parsed.as_array().unwrap();
    for item in items {
        let swift_code = item
            .get("swift_code")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        assert!(!swift_code.trim().is_empty());
    }
}

#[test]
fn metadata_contains_versioned_ir_contract() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let items = parsed.as_array().unwrap();

    for item in items {
        assert_eq!(
            item["schema_version"],
            cross_kit::VM_METADATA_SCHEMA_VERSION
        );
        let ir: cross_kit::VmMetadata =
            serde_json::from_value(item["ir"].clone()).expect("metadata ir should parse");
        assert_eq!(ir.schema_version, cross_kit::VM_METADATA_SCHEMA_VERSION);
        ir.validate().expect("metadata ir should be valid");
    }
}

#[test]
fn metadata_ir_matches_counter_list_snapshot() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let generated_ir = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["ir"].clone())
        .collect::<Vec<_>>();
    let expected_ir: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/metadata/counter-list.json"
    ))
    .unwrap();

    assert_eq!(generated_ir, expected_ir);
}
