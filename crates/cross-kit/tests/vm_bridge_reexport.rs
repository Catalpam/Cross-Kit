#![allow(dead_code)]

use std::sync::Arc;

trait DemoObserver: Send + Sync {
    fn on_state(&self, state: i32);
}

struct DemoViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl DemoViewModel {
    pub fn get_state(&self) -> i32 {
        1
    }

    pub fn subscribe(&self, observer: Arc<dyn DemoObserver>) -> i64 {
        drop(observer);
        7
    }
}

struct NoopDemoObserver;

impl DemoObserver for NoopDemoObserver {
    fn on_state(&self, _state: i32) {}
}

#[test]
fn vm_bridge_macro_works_without_importing_metadata_trait() {
    let vm = DemoViewModel;
    assert_eq!(vm.get_state(), 1);
    assert_eq!(vm.subscribe(Arc::new(NoopDemoObserver)), 7);

    let metadata: serde_json::Value =
        serde_json::from_str(<DemoViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata()).unwrap();

    assert_eq!(metadata["swift_bridge"], "DemoViewModelBridge");
    assert_eq!(
        metadata["schema_version"],
        cross_kit::VM_METADATA_SCHEMA_VERSION
    );
    assert_eq!(metadata["mode"], "state");
    assert_eq!(metadata["vm_type"], "DemoViewModel");
    assert_eq!(metadata["observer"], "DemoObserver");
    assert_eq!(metadata["observer_method"], "on_state");
    assert_eq!(metadata["state_type"], "i32");
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

trait LegacyObserver: Send + Sync {
    fn on_state(&self, state: i32);
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

    pub fn subscribe(&self, observer: Arc<dyn LegacyObserver>) -> i64 {
        drop(observer);
        8
    }
}

struct NoopLegacyObserver;

impl LegacyObserver for NoopLegacyObserver {
    fn on_state(&self, _state: i32) {}
}

#[test]
fn vm_bridge_macro_keeps_legacy_swift_bridge_attribute() {
    let vm = LegacyBridgeNameViewModel;
    assert_eq!(vm.get_state(), 2);
    assert_eq!(vm.subscribe(Arc::new(NoopLegacyObserver)), 8);

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

trait ExplicitBridgeObserver: Send + Sync {
    fn on_state(&self, state: i32);
}

struct ExplicitBridgeViewModel;

#[cross_kit::vm_bridge(bridge = "CustomCounterBridge", mode = "state")]
impl ExplicitBridgeViewModel {
    pub fn get_state(&self) -> i32 {
        3
    }

    pub fn subscribe(&self, observer: Arc<dyn ExplicitBridgeObserver>) -> i64 {
        drop(observer);
        9
    }
}

struct NoopExplicitBridgeObserver;

impl ExplicitBridgeObserver for NoopExplicitBridgeObserver {
    fn on_state(&self, _state: i32) {}
}

#[test]
fn vm_bridge_macro_keeps_explicit_bridge_override_while_inferring_other_fields() {
    let vm = ExplicitBridgeViewModel;
    assert_eq!(vm.get_state(), 3);
    assert_eq!(vm.subscribe(Arc::new(NoopExplicitBridgeObserver)), 9);

    let metadata: serde_json::Value = serde_json::from_str(
        <ExplicitBridgeViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata(),
    )
    .unwrap();

    assert_eq!(metadata["swift_bridge"], "CustomCounterBridge");
    assert_eq!(metadata["observer"], "ExplicitBridgeObserver");
    assert_eq!(metadata["observer_method"], "on_state");
    assert_eq!(metadata["state_type"], "i32");
    assert_eq!(metadata["ir"]["bridge_name"], "CustomCounterBridge");
}
