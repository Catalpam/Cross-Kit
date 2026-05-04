struct DemoViewModel;

#[cross_kit::vm_bridge(
    bridge = "DemoViewModelBridge",
    mode = "state",
    observer = "DemoObserver",
    observer_method = "on_state",
    state_type = "i32"
)]
impl DemoViewModel {
    pub fn get_state(&self) -> i32 {
        1
    }

    pub fn subscribe(&self, _observer: i32) -> i64 {
        7
    }
}

#[test]
fn vm_bridge_macro_works_without_importing_metadata_trait() {
    let vm = DemoViewModel;
    assert_eq!(vm.get_state(), 1);
    assert_eq!(vm.subscribe(0), 7);

    let metadata: serde_json::Value =
        serde_json::from_str(<DemoViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata()).unwrap();

    assert_eq!(metadata["swift_bridge"], "DemoViewModelBridge");
    assert_eq!(
        metadata["schema_version"],
        cross_kit::VM_METADATA_SCHEMA_VERSION
    );
    assert_eq!(metadata["mode"], "state");
    assert_eq!(metadata["vm_type"], "DemoViewModel");
    assert!(
        metadata["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "get_state")
    );

    let ir: cross_kit::VmMetadata = serde_json::from_value(metadata["ir"].clone()).unwrap();
    assert_eq!(ir.mode, cross_kit::VmMode::State);
    assert_eq!(ir.rust_type, "DemoViewModel");
    assert_eq!(ir.methods[0].return_type, "i32");
    ir.validate().unwrap();
}

struct LegacyBridgeNameViewModel;

#[cross_kit::vm_bridge(
    swift_bridge = "LegacyBridgeNameViewModelBridge",
    mode = "state",
    observer = "LegacyObserver",
    observer_method = "on_state",
    state_type = "i32"
)]
impl LegacyBridgeNameViewModel {
    pub fn get_state(&self) -> i32 {
        2
    }

    pub fn subscribe(&self, _observer: i32) -> i64 {
        8
    }
}

#[test]
fn vm_bridge_macro_keeps_legacy_swift_bridge_attribute() {
    let vm = LegacyBridgeNameViewModel;
    assert_eq!(vm.get_state(), 2);
    assert_eq!(vm.subscribe(0), 8);

    let metadata: serde_json::Value = serde_json::from_str(
        <LegacyBridgeNameViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata(),
    )
    .unwrap();

    assert_eq!(metadata["swift_bridge"], "LegacyBridgeNameViewModelBridge");
    assert_eq!(
        metadata["ir"]["bridge_name"],
        "LegacyBridgeNameViewModelBridge"
    );
}
