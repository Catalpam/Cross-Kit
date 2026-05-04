struct DemoViewModel;

#[cross_kit::vm_bridge(
    swift_bridge = "DemoViewModelBridge",
    mode = "state",
    observer = "DemoObserver",
    observer_method = "on_state",
    state_type = "DemoState"
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
    assert_eq!(metadata["mode"], "state");
    assert_eq!(metadata["vm_type"], "DemoViewModel");
    assert!(
        metadata["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "get_state")
    );
}
