use cross_kit_shared::{AppViewModel, CkVmMetadata, CounterViewModel, ListViewModel};

fn main() {
    println!("{}", build_metadata_json());
}

fn build_metadata_json() -> String {
    let metas = vec![
        AppViewModel::ck_vm_metadata(),
        CounterViewModel::ck_vm_metadata(),
        ListViewModel::ck_vm_metadata(),
    ];
    format!("[{}]", metas.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn metadata_json_is_valid() {
        let json = build_metadata_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn metadata_contains_swift_code() {
        let json = build_metadata_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
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
        let json = build_metadata_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
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
        let json = build_metadata_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
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
}
