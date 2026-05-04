use std::sync::Arc;

struct AppViewModel;
struct CounterState {
    value: i64,
}

trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(
    mode = "state",
    factory = AppViewModel::make_counter_vm,
    factory_type = "AppViewModel",
    factory_method = "make_counter_vm"
)]
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
    assert_eq!(metadata.factory.as_ref().unwrap().rust_type, "AppViewModel");
    assert_eq!(metadata.factory.unwrap().method, "make_counter_vm");
}
