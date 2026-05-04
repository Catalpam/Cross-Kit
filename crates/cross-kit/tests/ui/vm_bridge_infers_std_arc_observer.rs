trait CounterObserver: Send + Sync {
    fn on_state(&self, state: i32);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, observer: std::sync::Arc<dyn CounterObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn get_state(&self) -> i32 {
        0
    }
}

fn main() {
    let envelope: serde_json::Value =
        serde_json::from_str(<CounterViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata())
            .unwrap();
    let metadata: cross_kit::VmMetadata = serde_json::from_value(envelope["ir"].clone()).unwrap();
    assert_eq!(metadata.observer.unwrap().rust_type, "CounterObserver");
}
