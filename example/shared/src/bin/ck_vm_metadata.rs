use cross_kit_shared::{CkVmMetadata, CounterViewModel, ListViewModel};

fn main() {
    println!("{}", build_metadata_json());
}

fn build_metadata_json() -> String {
    let metas = vec![
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
}
