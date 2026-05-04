use std::sync::Arc;

struct AppViewModel;
struct CounterState {
    value: i64,
}

trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state", factory = AppViewModel::make_counter_vm)]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn get_state(&self) -> CounterState {
        CounterState { value: 1 }
    }
}

fn main() {
    let envelope: serde_json::Value =
        serde_json::from_str(<CounterViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata())
            .unwrap();
    let metadata: cross_kit::VmMetadata = serde_json::from_value(envelope["ir"].clone()).unwrap();
    let factory = metadata.factory.unwrap();
    assert_eq!(factory.rust_type, "AppViewModel");
    assert_eq!(factory.method, "make_counter_vm");
    assert_eq!(factory.bridge_name, "AppViewModelBridge");
}
