use std::sync::Arc;

trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

struct CounterState {
    value: i32,
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn get_state(&self) -> CounterState {
        CounterState { value: 0 }
    }
}

fn main() {
    let envelope: serde_json::Value =
        serde_json::from_str(<CounterViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata())
            .unwrap();
    let metadata: cross_kit::VmMetadata = serde_json::from_value(envelope["ir"].clone()).unwrap();
    assert_eq!(metadata.bridge_name, "CounterViewModelBridge");
    assert_eq!(metadata.state_type.as_deref(), Some("CounterState"));
    assert_eq!(metadata.observer.unwrap().rust_type, "CounterObserver");
}
