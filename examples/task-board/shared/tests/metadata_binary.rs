use serde_json::Value;
use std::process::Command;

fn run_metadata_binary() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ck_task_board_metadata"))
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
fn metadata_contains_task_board_contract() {
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
        serde_json::from_value(items[0]["ir"].clone()).expect("board metadata ir should parse");
    ir.validate().expect("metadata ir should be valid");
    assert_eq!(ir.rust_type, "TaskBoardViewModel");
    assert_eq!(ir.bridge_name, "TaskBoardViewModelBridge");
    assert_eq!(ir.state_type.as_deref(), Some("TaskBoardState"));
    assert!(ir.factory.is_none());

    let methods = ir
        .methods
        .iter()
        .map(|method| method.name.as_str())
        .collect::<Vec<_>>();
    assert!(methods.contains(&"new"));
    assert!(methods.contains(&"make_task_list_vm"));
    assert!(methods.contains(&"set_filter"));
    assert!(methods.contains(&"get_state"));
    assert!(methods.contains(&"subscribe"));
    assert!(methods.contains(&"unsubscribe"));

    let list_ir: cross_kit::VmMetadata =
        serde_json::from_value(items[1]["ir"].clone()).expect("list metadata ir should parse");
    list_ir
        .validate()
        .expect("list metadata ir should be valid");
    assert_eq!(list_ir.rust_type, "TaskListViewModel");
    assert_eq!(list_ir.mode, cross_kit::VmMode::DiffList);
    assert_eq!(list_ir.diff_type.as_deref(), Some("TaskDiff"));
    assert_eq!(list_ir.list_item_type.as_deref(), Some("TaskItem"));
    assert_eq!(
        list_ir
            .factory
            .as_ref()
            .map(|factory| factory.method.as_str()),
        Some("make_task_list_vm")
    );
}

#[test]
fn metadata_ir_matches_task_board_snapshot() {
    let json = run_metadata_binary();
    let parsed: Value = serde_json::from_str(json.trim()).unwrap();
    let generated_ir = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["ir"].clone())
        .collect::<Vec<_>>();
    let expected_ir: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/metadata/task-board.json"
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

    assert!(swift_code.contains("public final class TaskBoardViewModelBridge"));
    assert!(swift_code.contains("public func setFilter(filter: TaskFilter)"));
    let list_swift_code = parsed.as_array().unwrap()[1]
        .get("swift_code")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    assert!(list_swift_code.contains("public final class TaskListViewModelBridge"));
    assert!(list_swift_code.contains("public func addTask(title: String)"));
    assert!(list_swift_code.contains("public func renameTask(id: Int64, title: String)"));
    assert!(list_swift_code.contains("public func moveVisible(from: Int64, to: Int64)"));
}
